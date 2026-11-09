/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Transport between the cli and the daemon over a unix socket.

mod client;
mod server;

use std::{env, fs, io, os::unix::fs::MetadataExt, path::PathBuf};

pub(crate) use client::{send_mutation, send_query};
pub(crate) use server::{handle_client, start_server};

/// Path of the control socket, used by the daemon and the cli alike.
///
/// The `PIPELINK_SOCKET` variable overrides the path, which lets a second
/// daemon run next to an already running one. `XDG_RUNTIME_DIR` must not be
/// changed for that, `PipeWire` looks for its own socket there.
///
/// # Errors
/// Returns an error if the user id cannot be determined, which happens only
/// when `/proc` is not mounted.
pub(crate) fn socket_path() -> io::Result<PathBuf> {
    if let Some(path) = env::var_os("PIPELINK_SOCKET") {
        return Ok(PathBuf::from(path));
    }
    if let Some(dir) = env::var_os("XDG_RUNTIME_DIR") {
        return Ok(PathBuf::from(dir).join("pipelink.sock"));
    }
    let uid = fs::metadata("/proc/self")?.uid();
    Ok(env::temp_dir().join(format!("pipelink-{uid}.sock")))
}
