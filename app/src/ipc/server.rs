/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The daemon side of the control socket.

use std::{
    env, fs, io,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

use pipelink_common::events::{ControlEvent, ControlReply, StateEvent};
use pipelink_engine::executor::Envelope;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream, unix::OwnedWriteHalf},
    sync::{
        broadcast::{Receiver, error::RecvError},
        mpsc::Sender,
        oneshot,
    },
};
use tracing::warn;

use super::{current_uid, socket_path};

/// Binds the control socket the daemon listens on.
///
/// The socket lies in a directory that belongs to the user alone, so that no
/// other user can reach it. A socket that belongs to another user is neither
/// connected to nor removed.
///
/// # Errors
/// Returns an error if the socket path cannot be determined, if a daemon is
/// already listening on it, or if the socket cannot be created.
pub(crate) async fn start_server() -> io::Result<(UnixListener, SocketGuard)> {
    let socket = socket_path()?;

    prepare_socket_dir(&socket)?;

    match UnixStream::connect(&socket).await {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "A daemon is already running",
            ));
        },
        Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => {
            // The socket is the leftover of a daemon that is gone.
            remove_own_socket(&socket)?;
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {},
        Err(e) => return Err(e),
    }

    let listener = UnixListener::bind(&socket)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;

    Ok((listener, SocketGuard::new(socket)))
}

/// The socket of the daemon, removed when the daemon ends.
#[derive(Debug)]
pub(crate) struct SocketGuard {
    path: PathBuf,
}

impl SocketGuard {
    /// Takes the path of the socket that is removed when the guard is dropped.
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Makes sure that the directory of the socket belongs to this user alone.
///
/// The directory is created when it is not there yet. A directory that belongs
/// to another user is never used, because the socket in it could be reached by
/// everybody.
fn prepare_socket_dir(socket: &Path) -> io::Result<()> {
    // A path that was set by hand is used as it is.
    if env::var_os("PIPELINK_SOCKET").is_some() {
        return Ok(());
    }

    let Some(dir) = socket.parent() else {
        return Ok(());
    };

    match fs::DirBuilder::new().mode(0o700).create(dir) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {},
        Err(e) => return Err(e),
    }

    // The directory is there already. It has to belong to this user, or it is
    // the directory of somebody else.
    let metadata = fs::metadata(dir)?;
    if metadata.uid() != current_uid()? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "the directory of the socket belongs to another user: {}",
                dir.display()
            ),
        ));
    }

    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;

    Ok(())
}

/// Removes the socket of a daemon that is gone.
///
/// A socket that belongs to another user is never removed, because it is not
/// the socket of this daemon.
fn remove_own_socket(socket: &Path) -> io::Result<()> {
    match fs::symlink_metadata(socket) {
        Ok(metadata) => {
            if metadata.uid() != current_uid()? {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("the socket belongs to another user: {}", socket.display()),
                ));
            }
            fs::remove_file(socket)?;
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {},
        Err(e) => return Err(e),
    }

    Ok(())
}

/// Handles one client connection.
///
/// # Errors
/// Returns an error if reading the request or writing the response fails.
pub(crate) async fn handle_client(
    socket: UnixStream,
    control_tx: Sender<Envelope>,
    mut state_rx: Receiver<StateEvent>,
) -> io::Result<()> {
    let (read_half, mut write_half) = socket.into_split();
    let mut reader = BufReader::new(read_half);

    let mut line = String::new();
    if reader.read_line(&mut line).await? == 0 {
        return Ok(());
    }

    let request: ControlEvent =
        serde_json::from_str(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let (reply_tx, reply_rx) = oneshot::channel();
    control_tx
        .send(Envelope {
            event: request,
            reply: Some(reply_tx),
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))?;

    match reply_rx.await {
        // The executor answered right away, so this was a query.
        Ok(reply) => {
            write_ctrl_reply(&mut write_half, &reply).await?;
        },
        // The executor dropped the reply, so this was a state change and the
        // client waits for the matching state event.
        Err(_) => loop {
            tokio::select! {
                event = state_rx.recv() => match event {
                    Ok(event) => write_state_reply(&mut write_half, &event).await?,
                    Err(RecvError::Lagged(missed)) => warn!("{missed} missed state events"),
                    Err(RecvError::Closed) => break,
                },
                // The client closes the connection once it has its event, so
                // the read tells us when to stop forwarding.
                read = reader.read_line(&mut line) => {
                    if read.unwrap_or(0) == 0 {
                        break;
                    }
                },
            }
        },
    }

    Ok(())
}

/// Writes the answer to a query as one json line.
///
/// # Errors
/// Returns an error if the answer cannot be serialized or written.
async fn write_ctrl_reply(writer: &mut OwnedWriteHalf, reply: &ControlReply) -> io::Result<()> {
    write_json_line(writer, reply).await
}

/// Writes a state event as one json line.
///
/// # Errors
/// Returns an error if the event cannot be serialized or written.
async fn write_state_reply(writer: &mut OwnedWriteHalf, state: &StateEvent) -> io::Result<()> {
    write_json_line(writer, state).await
}

/// Writes one json line to the client.
///
/// # Errors
/// Returns an error if the value cannot be serialized or written.
async fn write_json_line<T: serde::Serialize>(
    writer: &mut OwnedWriteHalf,
    value: &T,
) -> io::Result<()> {
    let payload =
        serde_json::to_string(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writer.write_all(payload.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}
