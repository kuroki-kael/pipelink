/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Independent GUI process launching from the daemon tray controller.
//!
//! Launches `pipelink --gui` detached from the daemon's process group (and, when
//! running under a `systemd` unit, in its own transient `--user` unit) so restarting
//! or stopping the daemon never terminates an open mixer window.

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    env,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use tracing::{debug, warn};

const GUI_SYSTEMD_UNIT: &str = "pipelink-gui";
const FORWARDED_GUI_ENV: &[&str] = &[
    "WAYLAND_DISPLAY",
    "DISPLAY",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_DATA_DIRS",
    "QT_QPA_PLATFORM",
];

pub(super) fn launch_independent_gui(gui_child: &Arc<Mutex<Option<Child>>>) {
    if is_tracked_child_running(gui_child) {
        debug!("GUI window already running");
        return;
    }

    let Ok(exe) = env::current_exe() else {
        warn!("Cannot launch PipeLink GUI from tray: current_exe failed");
        return;
    };

    if env::var_os("INVOCATION_ID").is_some() {
        let child_slot = Arc::clone(gui_child);
        thread::spawn(move || {
            if is_systemd_gui_active() {
                debug!("GUI systemd unit already running");
                return;
            }
            if try_launch_via_systemd_run(&exe) {
                debug!("Launched PipeLink GUI in independent systemd user unit");
                return;
            }
            spawn_detached_child(&exe, &child_slot);
        });
        return;
    }

    spawn_detached_child(&exe, gui_child);
}

fn is_tracked_child_running(gui_child: &Arc<Mutex<Option<Child>>>) -> bool {
    let Ok(mut guard) = gui_child.lock() else {
        return false;
    };
    let Some(child) = guard.as_mut() else {
        return false;
    };
    if let Ok(None) = child.try_wait() {
        true
    } else {
        *guard = None;
        false
    }
}

fn is_systemd_gui_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", GUI_SYSTEMD_UNIT])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn try_launch_via_systemd_run(exe: &Path) -> bool {
    let mut cmd = build_systemd_run_command(exe);
    cmd.status().is_ok_and(|status| status.success())
}

pub(super) fn build_systemd_run_command(exe: &Path) -> Command {
    let mut cmd = Command::new("systemd-run");
    cmd.args([
        "--user",
        "--collect",
        "--quiet",
        "--unit",
        GUI_SYSTEMD_UNIT,
        "--description",
        "PipeLink Mixer GUI",
    ]);
    for var in FORWARDED_GUI_ENV {
        if env::var_os(var).is_some() {
            cmd.arg(format!("--setenv={var}"));
        }
    }
    cmd.arg("--").arg(exe).arg("--gui");
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

pub(super) fn build_detached_gui_command(exe: &Path) -> Command {
    let mut cmd = Command::new(exe);
    cmd.arg("--gui")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    cmd.process_group(0);
    cmd
}

fn spawn_detached_child(exe: &Path, gui_child: &Arc<Mutex<Option<Child>>>) {
    match build_detached_gui_command(exe).spawn() {
        Ok(child) => {
            if let Ok(mut guard) = gui_child.lock() {
                *guard = Some(child);
            }
            debug!("Launched detached PipeLink GUI process from tray icon");
        },
        Err(err) => warn!("Failed to launch PipeLink GUI from tray icon: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_builders_configure_gui_flag_and_independent_unit() {
        let exe = Path::new("/usr/bin/pipelink");

        let detached = build_detached_gui_command(exe);
        assert_eq!(detached.get_program(), exe);
        let detached_args: Vec<_> = detached.get_args().collect();
        assert_eq!(detached_args, ["--gui"]);

        let systemd = build_systemd_run_command(exe);
        assert_eq!(systemd.get_program(), "systemd-run");
        let args: Vec<String> = systemd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--user".to_string()));
        assert!(args.contains(&"--collect".to_string()));
        assert!(args.contains(&GUI_SYSTEMD_UNIT.to_string()));
        assert!(args.ends_with(&[
            "--".to_string(),
            exe.display().to_string(),
            "--gui".to_string()
        ]));
    }
}
