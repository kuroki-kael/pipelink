/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The cli side of the control socket.

use std::{io, time::Duration};

use pipelink_common::events::{ControlEvent, ControlReply, StateEvent};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use super::socket_path;

/// How long the client waits for the answer to a query.
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

/// How long the client waits for the event that confirms a state change.
const MUTATION_TIMEOUT: Duration = Duration::from_secs(10);

/// Connects to the control socket of the running daemon.
///
/// # Errors
/// Returns an error if the socket path cannot be determined or the daemon is
/// not listening.
pub(crate) async fn connect_to_daemon() -> io::Result<UnixStream> {
    let socket = socket_path()?;
    UnixStream::connect(socket).await
}

/// Sends a query to the daemon and returns its answer.
///
/// # Errors
/// Returns an error if the daemon cannot be reached, does not answer in time
/// or sends an answer that cannot be parsed.
pub(crate) async fn send_query(request: ControlEvent) -> io::Result<ControlReply> {
    // The write half has to stay alive while the answer is read, otherwise
    // the daemon sees the connection close before it answers.
    let (mut reader, _write_half) = open_request(request).await?;
    let line = read_line(&mut reader, QUERY_TIMEOUT).await?;
    serde_json::from_str(&line).map_err(|e| unexpected_answer(&line, &e))
}

/// Sends a state change to the daemon and waits for the event that confirms it.
///
/// The daemon forwards every state event it sees, so the caller decides which
/// one belongs to its request.
///
/// # Errors
/// Returns an error if the daemon cannot be reached, sends nothing in time or
/// sends an event that cannot be parsed.
pub(crate) async fn send_mutation(
    request: ControlEvent,
    is_confirmation: impl Fn(&StateEvent) -> bool,
) -> io::Result<StateEvent> {
    let (mut reader, _write_half) = open_request(request).await?;
    loop {
        let line = read_line(&mut reader, MUTATION_TIMEOUT).await?;
        let event: StateEvent =
            serde_json::from_str(&line).map_err(|e| unexpected_answer(&line, &e))?;
        if is_confirmation(&event) {
            return Ok(event);
        }
    }
}

/// Sends a request to the daemon and splits the connection for reading.
async fn open_request(
    request: ControlEvent,
) -> io::Result<(BufReader<OwnedReadHalf>, OwnedWriteHalf)> {
    let mut stream = connect_to_daemon().await?;
    let payload = serde_json::to_string(&request)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    stream.write_all(payload.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;

    let (read_half, write_half) = stream.into_split();
    Ok((BufReader::new(read_half), write_half))
}

/// Reads a single line from the daemon.
///
/// # Errors
/// Returns an error if the daemon sends nothing within the timeout or closes
/// the connection.
async fn read_line(reader: &mut BufReader<OwnedReadHalf>, timeout: Duration) -> io::Result<String> {
    let mut line = String::new();
    tokio::time::timeout(timeout, reader.read_line(&mut line))
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "the daemon did not answer within the timeout",
            )
        })??;

    if line.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the daemon closed the connection",
        ));
    }

    Ok(line)
}

/// Turns a parse error into one that names the answer the daemon sent.
fn unexpected_answer(line: &str, error: &serde_json::Error) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("unexpected answer '{line}': {error}"),
    )
}
