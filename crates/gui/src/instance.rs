/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Single-instance socket guard and focus-activation handoff for `pipelink-gui`.
//!
//! Uses `$XDG_RUNTIME_DIR/pipelink/pipelink-gui.sock` (alongside the daemon control
//! socket) so duplicate launches from desktop launchers, terminals, or tray clicks
//! activate the already-open window instead of spawning a second GUI window.

use std::{
    fs,
    io::{self, ErrorKind, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::Mutex,
};

use tracing::{debug, info, warn};

const GUI_SOCKET_NAME: &str = "pipelink-gui.sock";

static ACTIVE_LISTENER: Mutex<Option<UnixListener>> = Mutex::new(None);

/// Outcome of acquiring the single-instance GUI socket lock.
#[derive(Debug)]
pub(crate) enum InstanceLock {
    /// This process owns the primary GUI instance and removes its socket on drop.
    Primary(InstanceGuard),
    /// Another GUI instance is already running and has been asked to focus its window.
    ActivatedExisting,
}

/// RAII handle cleaning up the single-instance Unix socket when the GUI exits.
#[derive(Debug)]
pub(crate) struct InstanceGuard {
    path: PathBuf,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Ensures only one `pipelink-gui` instance runs per daemon socket namespace.
///
/// If another instance is already listening, signals it to focus and returns
/// [`InstanceLock::ActivatedExisting`]. Stale socket files left by previous crashes
/// are removed automatically. On unexpected I/O errors binding the socket, falls back
/// to allowing startup cleanly without blocking the UI.
pub(crate) fn acquire_or_activate() -> Option<InstanceGuard> {
    let Ok(sock_path) = gui_socket_path() else {
        return None;
    };

    match acquire_at(&sock_path) {
        Ok(InstanceLock::Primary(guard)) => Some(guard),
        Ok(InstanceLock::ActivatedExisting) => {
            info!("Existing PipeLink GUI instance activated");
            std::process::exit(0);
        },
        Err(err) => {
            warn!("Single-instance GUI lock unavailable, continuing anyway: {err}");
            None
        },
    }
}

/// Takes the bound std [`UnixListener`] once so the async subscription can await
/// activation signals from secondary launches.
pub(crate) fn take_activation_listener() -> Option<UnixListener> {
    ACTIVE_LISTENER.lock().ok().and_then(|mut g| g.take())
}

pub(crate) fn acquire_at(sock_path: &Path) -> io::Result<InstanceLock> {
    if try_activate_existing(sock_path)? {
        return Ok(InstanceLock::ActivatedExisting);
    }

    if let Some(parent) = sock_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let listener = match UnixListener::bind(sock_path) {
        Ok(listener) => listener,
        Err(err) if err.kind() == ErrorKind::AddrInUse => {
            if try_activate_existing(sock_path)? {
                return Ok(InstanceLock::ActivatedExisting);
            }
            let _ = fs::remove_file(sock_path);
            UnixListener::bind(sock_path)?
        },
        Err(err) => return Err(err),
    };

    if let Ok(mut guard) = ACTIVE_LISTENER.lock() {
        *guard = Some(listener);
    }
    debug!(
        "Bound GUI single-instance socket at {}",
        sock_path.display()
    );

    Ok(InstanceLock::Primary(InstanceGuard {
        path: sock_path.to_path_buf(),
    }))
}

fn try_activate_existing(sock_path: &Path) -> io::Result<bool> {
    match UnixStream::connect(sock_path) {
        Ok(mut stream) => {
            let _ = stream.write_all(b"1");
            Ok(true)
        },
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::ConnectionRefused | ErrorKind::NotFound | ErrorKind::ConnectionReset
            ) =>
        {
            if sock_path.exists() {
                let _ = fs::remove_file(sock_path);
            }
            Ok(false)
        },
        Err(err) => Err(err),
    }
}

fn gui_socket_path() -> io::Result<PathBuf> {
    let daemon_sock = pipelink_ipc::socket_path()?;
    Ok(daemon_sock.with_file_name(GUI_SOCKET_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_instance_socket_activates_existing_and_recovers_stale_file() {
        let dir = std::env::temp_dir().join(format!("pipelink-gui-inst-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let sock = dir.join(GUI_SOCKET_NAME);

        // Stale non-listening file is replaced cleanly by the first primary acquire.
        fs::create_dir_all(&dir).ok();
        fs::write(&sock, b"stale").ok();

        let first = acquire_at(&sock);
        let Ok(InstanceLock::Primary(guard)) = first else {
            panic!("expected primary instance, got {first:?}");
        };
        assert!(sock.exists());
        let listener = take_activation_listener();
        assert!(listener.is_some());

        // Second launch detects the active listener and signals it instead of binding.
        let second = acquire_at(&sock);
        assert!(matches!(second, Ok(InstanceLock::ActivatedExisting)));

        drop(listener);
        drop(guard);
        assert!(!sock.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
