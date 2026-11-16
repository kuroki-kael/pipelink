/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Runtime socket path discovery and user ownership helpers.

use std::{env, fs, io, os::unix::fs::MetadataExt, path::PathBuf};

/// Directory name under `XDG_RUNTIME_DIR` holding the daemon socket.
const SOCKET_DIR_NAME: &str = "pipelink";

/// File name of the daemon Unix control socket.
const SOCKET_FILE_NAME: &str = "pipelink.sock";

/// Resolves the path of the daemon control socket, shared by daemon and clients.
///
/// `PIPELINK_SOCKET` overrides the computed path so multiple instances can coexist
/// without altering `XDG_RUNTIME_DIR`.
///
/// # Errors
/// Returns an error if the current user id cannot be determined when falling back to
/// `temp_dir`.
pub fn socket_path() -> io::Result<PathBuf> {
    if let Some(path) = env::var_os("PIPELINK_SOCKET") {
        return Ok(PathBuf::from(path));
    }
    Ok(socket_dir()?.join(SOCKET_FILE_NAME))
}

/// Resolves the directory containing the daemon control socket.
///
/// # Errors
/// Returns an error if `/proc/self` cannot be inspected when `XDG_RUNTIME_DIR` is unset.
pub(crate) fn socket_dir() -> io::Result<PathBuf> {
    if let Some(dir) = env::var_os("XDG_RUNTIME_DIR") {
        return Ok(PathBuf::from(dir).join(SOCKET_DIR_NAME));
    }
    let uid = current_uid()?;
    Ok(env::temp_dir().join(format!("{SOCKET_DIR_NAME}-{uid}")))
}

/// Returns the numeric UID of the running process.
///
/// # Errors
/// Returns an error if `/proc/self` metadata cannot be read.
pub(crate) fn current_uid() -> io::Result<u32> {
    Ok(fs::metadata("/proc/self")?.uid())
}
