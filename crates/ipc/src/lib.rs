/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Unix domain socket IPC transport, daemon server, and client abstractions for
//! `PipeLink`.

pub mod client;
pub mod server;
pub mod socket;

#[cfg(test)]
mod tests;

pub use client::{ControlClient, StateSubscription, UnixSocketClient};
pub use server::{Envelope, SocketGuard, handle_client, start_server, start_server_at};
pub use socket::socket_path;
