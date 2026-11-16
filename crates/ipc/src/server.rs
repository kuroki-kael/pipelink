/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Daemon-side Unix domain socket listener and client handler.

use std::{
    env, fs, io,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

use pipelink_common::events::{ControlEvent, ControlReply, StateEvent};
use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        UnixListener, UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::{
        broadcast::{Receiver, error::RecvError},
        mpsc::Sender,
        oneshot,
    },
};
use tracing::warn;

use crate::socket::{current_uid, socket_path};

/// Control request paired with an optional one-shot reply channel for queries.
#[derive(Debug)]
pub struct Envelope {
    /// Requested state change or query.
    pub event: ControlEvent,
    /// Reply channel for query commands, `None` when no direct reply is expected.
    pub reply: Option<oneshot::Sender<ControlReply>>,
}

/// Guard removing the bound daemon Unix socket when dropped.
#[derive(Debug)]
pub struct SocketGuard {
    path: PathBuf,
}

impl SocketGuard {
    /// Creates a guard tracking `path`.
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Binds the daemon control socket at the default XDG / `PIPELINK_SOCKET` location.
///
/// # Errors
/// Returns an error if the path cannot be determined, another daemon is listening, or
/// binding the socket fails.
pub async fn start_server() -> io::Result<(UnixListener, SocketGuard)> {
    let socket = socket_path()?;
    start_server_at(&socket).await
}

/// Binds the daemon control socket at `socket`, validating directory and socket
/// ownership.
///
/// # Errors
/// Returns an error if directory permissions fail, another daemon is listening, or
/// binding fails.
pub async fn start_server_at(socket: &Path) -> io::Result<(UnixListener, SocketGuard)> {
    prepare_socket_dir(socket)?;

    match UnixStream::connect(socket).await {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "A daemon is already running",
            ));
        },
        Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => {
            remove_own_socket(socket)?;
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {},
        Err(e) => return Err(e),
    }

    let listener = UnixListener::bind(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;

    Ok((listener, SocketGuard::new(socket.to_path_buf())))
}

/// Ensures the parent directory of `socket` exists and is owned exclusively by this user.
fn prepare_socket_dir(socket: &Path) -> io::Result<()> {
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

/// Removes a stale socket file only if it belongs to the current user.
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

/// Handles one incoming IPC client connection.
///
/// # Errors
/// Returns an error if reading the request or writing responses fails.
pub async fn handle_client(
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

    if matches!(request, ControlEvent::SubscribeState) {
        return forward_state_events(&mut reader, &mut write_half, &mut state_rx).await;
    }

    let (reply_tx, reply_rx) = oneshot::channel();
    control_tx
        .send(Envelope {
            event: request,
            reply: Some(reply_tx),
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))?;

    match reply_rx.await {
        Ok(reply) => write_json_line(&mut write_half, &reply).await?,
        Err(_) => forward_state_events(&mut reader, &mut write_half, &mut state_rx).await?,
    }

    Ok(())
}

/// Streams broadcast [`StateEvent`] lines until the client closes or the channel shuts
/// down.
async fn forward_state_events(
    reader: &mut BufReader<OwnedReadHalf>,
    write_half: &mut OwnedWriteHalf,
    state_rx: &mut Receiver<StateEvent>,
) -> io::Result<()> {
    let mut line = String::new();
    loop {
        tokio::select! {
            event = state_rx.recv() => match event {
                Ok(event) => write_json_line(write_half, &event).await?,
                Err(RecvError::Lagged(missed)) => warn!("{missed} missed state events"),
                Err(RecvError::Closed) => break,
            },
            read = reader.read_line(&mut line) => {
                if read.unwrap_or(0) == 0 {
                    break;
                }
            },
        }
    }
    Ok(())
}

/// Serializes one response or state event as a single JSON line.
async fn write_json_line<T: Serialize>(writer: &mut OwnedWriteHalf, value: &T) -> io::Result<()> {
    let json =
        serde_json::to_string(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}
