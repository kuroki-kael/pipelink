/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! IPC client trait, Unix socket implementation, and live state subscription stream.

use std::{
    future::Future,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use pipelink_common::events::{ControlEvent, ControlReply, StateEvent};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use crate::socket::socket_path;

/// Timeout while waiting for a query response from the daemon.
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

/// Timeout while waiting for a state mutation confirmation from the daemon.
const MUTATION_TIMEOUT: Duration = Duration::from_secs(10);

/// Asynchronous client abstraction for querying, mutating, and subscribing to the daemon.
pub trait ControlClient: Send + Sync {
    /// Sends a query request and awaits the matching [`ControlReply`].
    fn query(&self, request: ControlEvent)
    -> impl Future<Output = io::Result<ControlReply>> + Send;

    /// Sends a state-changing control request and waits until `is_confirmation` matches.
    fn mutate<F>(
        &self,
        request: ControlEvent,
        is_confirmation: F,
    ) -> impl Future<Output = io::Result<StateEvent>> + Send
    where
        F: Fn(&StateEvent) -> bool + Send;

    /// Opens a live stream of [`StateEvent`] notifications broadcast by the daemon.
    fn subscribe(&self) -> impl Future<Output = io::Result<StateSubscription>> + Send;
}

/// Persistent subscription receiving real-time [`StateEvent`] updates from the daemon.
#[derive(Debug)]
pub struct StateSubscription {
    reader: BufReader<OwnedReadHalf>,
    // Keep the write half alive so the daemon does not observe an immediate EOF.
    _write_half: OwnedWriteHalf,
}

impl StateSubscription {
    /// Reads the next [`StateEvent`] from the daemon, returning `Ok(None)` when closed
    /// cleanly.
    ///
    /// # Errors
    /// Returns an error if reading from the socket fails or an event cannot be parsed.
    pub async fn next_event(&mut self) -> io::Result<Option<StateEvent>> {
        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line).await?;
        if bytes == 0 {
            return Ok(None);
        }
        let event = serde_json::from_str(&line).map_err(|e| unexpected_answer(&line, &e))?;
        Ok(Some(event))
    }
}

/// [`ControlClient`] backed by the daemon's local Unix domain socket.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnixSocketClient {
    custom_path: Option<PathBuf>,
}

impl UnixSocketClient {
    /// Creates a client targeting the default XDG / `PIPELINK_SOCKET` path.
    #[must_use]
    pub const fn new() -> Self {
        Self { custom_path: None }
    }

    /// Creates a client targeting an explicit socket path.
    #[must_use]
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            custom_path: Some(path.into()),
        }
    }

    /// Resolves the socket path used by this client.
    ///
    /// # Errors
    /// Returns an error if default socket path discovery fails.
    pub fn socket_path(&self) -> io::Result<PathBuf> {
        match &self.custom_path {
            Some(path) => Ok(path.clone()),
            None => socket_path(),
        }
    }

    /// Connects directly to the daemon Unix socket.
    ///
    /// # Errors
    /// Returns an error if the socket path cannot be resolved or the daemon is not
    /// listening.
    pub async fn connect(&self) -> io::Result<UnixStream> {
        let path = self.socket_path()?;
        UnixStream::connect(path).await
    }

    /// Opens a request stream by writing one serialized [`ControlEvent`] line.
    async fn open_request(
        &self,
        request: &ControlEvent,
    ) -> io::Result<(BufReader<OwnedReadHalf>, OwnedWriteHalf)> {
        let path = self.socket_path()?;
        open_request_at(&path, request).await
    }
}

impl ControlClient for UnixSocketClient {
    async fn query(&self, request: ControlEvent) -> io::Result<ControlReply> {
        let (mut reader, _write_half) = self.open_request(&request).await?;
        let line = read_line(&mut reader, QUERY_TIMEOUT).await?;
        serde_json::from_str(&line).map_err(|e| unexpected_answer(&line, &e))
    }

    async fn mutate<F>(&self, request: ControlEvent, is_confirmation: F) -> io::Result<StateEvent>
    where
        F: Fn(&StateEvent) -> bool + Send,
    {
        let (mut reader, _write_half) = self.open_request(&request).await?;
        loop {
            let line = read_line(&mut reader, MUTATION_TIMEOUT).await?;
            let event: StateEvent =
                serde_json::from_str(&line).map_err(|e| unexpected_answer(&line, &e))?;
            if is_confirmation(&event) {
                return Ok(event);
            }
        }
    }

    async fn subscribe(&self) -> io::Result<StateSubscription> {
        let (reader, write_half) = self.open_request(&ControlEvent::SubscribeState).await?;
        Ok(StateSubscription {
            reader,
            _write_half: write_half,
        })
    }
}

/// Sends a query using the default [`UnixSocketClient`].
///
/// # Errors
/// Returns an error if connecting, sending, or parsing the reply fails.
pub async fn send_query(request: ControlEvent) -> io::Result<ControlReply> {
    UnixSocketClient::new().query(request).await
}

/// Sends a mutation request using the default [`UnixSocketClient`].
///
/// # Errors
/// Returns an error if connecting, sending, or waiting for confirmation fails.
pub async fn send_mutation(
    request: ControlEvent,
    is_confirmation: impl Fn(&StateEvent) -> bool + Send,
) -> io::Result<StateEvent> {
    UnixSocketClient::new()
        .mutate(request, is_confirmation)
        .await
}

/// Connects to `path`, writes `request` as one JSON line, and splits the stream.
async fn open_request_at(
    path: &Path,
    request: &ControlEvent,
) -> io::Result<(BufReader<OwnedReadHalf>, OwnedWriteHalf)> {
    let mut stream = UnixStream::connect(path).await?;
    let payload = serde_json::to_string(request)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    stream.write_all(payload.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;

    let (read_half, write_half) = stream.into_split();
    Ok((BufReader::new(read_half), write_half))
}

/// Reads a single line from the daemon within `timeout`.
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

/// Formats a parse failure mentioning the daemon payload.
fn unexpected_answer(line: &str, error: &serde_json::Error) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("unexpected answer '{line}': {error}"),
    )
}
