/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The daemon side of the control socket.

use std::{fs, io, os::unix::fs::PermissionsExt};

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

use super::socket_path;

/// Binds the control socket the daemon listens on.
///
/// # Errors
/// Returns an error if the socket path cannot be determined, if a daemon is
/// already listening on it, or if the socket cannot be created.
pub(crate) async fn start_server() -> io::Result<UnixListener> {
    let socket = socket_path()?;

    match UnixStream::connect(&socket).await {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "A daemon is already running",
            ));
        },
        Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => {
            fs::remove_file(&socket)?;
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {},
        Err(e) => return Err(e),
    }

    let listener = UnixListener::bind(&socket)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;

    Ok(listener)
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
                    Err(RecvError::Lagged(missed)) => eprintln!("{missed} missed state events"),
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
