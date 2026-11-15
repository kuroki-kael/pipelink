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

/// The name of the directory that holds the socket of the daemon.
const SOCKET_DIR_NAME: &str = "pipelink";

/// The name of the socket of the daemon.
const SOCKET_FILE_NAME: &str = "pipelink.sock";

/// The path of the socket of the daemon, used by the daemon and the cli alike.
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
    Ok(socket_dir()?.join(SOCKET_FILE_NAME))
}

/// The directory that holds the socket of the daemon.
///
/// The directory belongs to the user alone, so that no other user can reach
/// the socket of the daemon. It lies in the runtime directory of the user, or
/// in the temporary directory when there is none.
///
/// # Errors
/// Returns an error if the user id cannot be determined, which happens only
/// when `/proc` is not mounted.
pub(crate) fn socket_dir() -> io::Result<PathBuf> {
    if let Some(dir) = env::var_os("XDG_RUNTIME_DIR") {
        return Ok(PathBuf::from(dir).join(SOCKET_DIR_NAME));
    }
    let uid = current_uid()?;
    Ok(env::temp_dir().join(format!("{SOCKET_DIR_NAME}-{uid}")))
}

/// The id of the user that runs the process.
///
/// # Errors
/// Returns an error if the user id cannot be determined, which happens only
/// when `/proc` is not mounted.
pub(crate) fn current_uid() -> io::Result<u32> {
    Ok(fs::metadata("/proc/self")?.uid())
}
