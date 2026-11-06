/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::path::{Path, PathBuf};

use pipelink_common::types::AppId;

use crate::{
    desktop::DesktopDatabase,
    proc::ProcessInfo,
    wrappers::{UnwrappedApp, WrapperKind},
};

const INTERPRETERS: &[&str] = &[
    "bash", "sh", "dash", "zsh", "python", "python2", "python3", "ruby", "perl", "node", "nodejs",
];

/// Inspects a process to determine if it is a script interpreter, Snap, `AppImage`, or
/// Java app.
#[must_use]
pub fn inspect_script_or_package(
    proc: &ProcessInfo,
    _ancestry: &[ProcessInfo],
    desktop_db: &DesktopDatabase,
) -> Option<UnwrappedApp> {
    // 1. Check Snap packaging
    if let Some(snap_name) = proc.environ.get("SNAP_NAME") {
        return Some(build_snap_app(proc, snap_name, desktop_db));
    }

    // 2. Check AppImage
    if let Some(appimage_path) = proc.environ.get("APPIMAGE") {
        return Some(build_appimage_app(proc, appimage_path, desktop_db));
    }

    // 3. Check Java runtime
    if let Some(bin) = proc.binary_name()
        && (bin == "java" || bin == "javaw")
    {
        return Some(build_java_app(proc, desktop_db));
    }

    // 4. Check Script Interpreters
    if let Some(bin) = proc.binary_name()
        && INTERPRETERS.contains(&bin)
    {
        return build_script_app(proc, desktop_db);
    }

    None
}

fn build_snap_app(
    proc: &ProcessInfo,
    snap_name: &str,
    desktop_db: &DesktopDatabase,
) -> UnwrappedApp {
    let desktop_entry = desktop_db
        .find_by_id(&AppId::from(format!("{snap_name}_{snap_name}")))
        .or_else(|| desktop_db.find_best_match(snap_name));

    let display_name = desktop_entry
        .map(|e| e.name.clone())
        .or_else(|| Some(capitalize_name(snap_name)));

    UnwrappedApp {
        target_pid: proc.pid,
        app_name: snap_name.to_lowercase(),
        display_name,
        wrapper: WrapperKind::Snap,
        binary_path: proc.exe.clone(),
        app_id: Some(AppId::from(snap_name)),
        icon: desktop_entry.and_then(|e| e.icon.clone()),
        desktop_file: desktop_entry.map(|e| e.path.clone()),
        categories: desktop_entry
            .map(|e| e.categories.clone())
            .unwrap_or_default(),
        generic_name: desktop_entry.and_then(|e| e.generic_name.clone()),
        comment: desktop_entry.and_then(|e| e.comment.clone()),
    }
}

fn build_appimage_app(
    proc: &ProcessInfo,
    appimage_path: &str,
    desktop_db: &DesktopDatabase,
) -> UnwrappedApp {
    let path = Path::new(appimage_path);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("appimage");

    let clean_name = stem
        .split('-')
        .next()
        .unwrap_or(stem)
        .split('_')
        .next()
        .unwrap_or(stem);

    let desktop_entry = desktop_db.find_best_match(clean_name);

    let display_name = desktop_entry
        .map(|e| e.name.clone())
        .or_else(|| Some(capitalize_name(clean_name)));

    UnwrappedApp {
        target_pid: proc.pid,
        app_name: clean_name.to_lowercase(),
        display_name,
        wrapper: WrapperKind::AppImage,
        binary_path: Some(PathBuf::from(appimage_path)),
        app_id: None,
        icon: desktop_entry.and_then(|e| e.icon.clone()),
        desktop_file: desktop_entry.map(|e| e.path.clone()),
        categories: desktop_entry
            .map(|e| e.categories.clone())
            .unwrap_or_default(),
        generic_name: desktop_entry.and_then(|e| e.generic_name.clone()),
        comment: desktop_entry.and_then(|e| e.comment.clone()),
    }
}

fn build_java_app(proc: &ProcessInfo, desktop_db: &DesktopDatabase) -> UnwrappedApp {
    let mut jar_name = None;

    for (i, arg) in proc.cmdline.iter().enumerate() {
        if arg == "-jar" {
            if let Some(next) = proc.cmdline.get(i + 1) {
                let path = Path::new(next);
                jar_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(ToString::to_string);
                break;
            }
        } else if arg.ends_with(".jar") {
            let path = Path::new(arg);
            jar_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(ToString::to_string);
            break;
        }
    }

    let app_name = jar_name.unwrap_or_else(|| "java".to_string());
    let desktop_entry = desktop_db.find_best_match(&app_name);

    let display_name = desktop_entry
        .map(|e| e.name.clone())
        .or_else(|| Some(capitalize_name(&app_name)));

    UnwrappedApp {
        target_pid: proc.pid,
        app_name: app_name.to_lowercase(),
        display_name,
        wrapper: WrapperKind::Java,
        binary_path: proc.exe.clone(),
        app_id: None,
        icon: desktop_entry.and_then(|e| e.icon.clone()),
        desktop_file: desktop_entry.map(|e| e.path.clone()),
        categories: desktop_entry
            .map(|e| e.categories.clone())
            .unwrap_or_default(),
        generic_name: desktop_entry.and_then(|e| e.generic_name.clone()),
        comment: desktop_entry.and_then(|e| e.comment.clone()),
    }
}

fn build_script_app(proc: &ProcessInfo, desktop_db: &DesktopDatabase) -> Option<UnwrappedApp> {
    let mut script_path = None;
    for arg in proc.cmdline.iter().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        script_path = Some(arg.as_str());
        break;
    }

    let script = script_path?;
    let path = Path::new(script);
    let stem = path.file_stem()?.to_str()?;

    let app_name = stem.to_string();
    let desktop_entry = desktop_db.find_best_match(&app_name);

    let display_name = desktop_entry
        .map(|e| e.name.clone())
        .or_else(|| Some(capitalize_name(&app_name)));

    Some(UnwrappedApp {
        target_pid: proc.pid,
        app_name: app_name.to_lowercase(),
        display_name,
        wrapper: WrapperKind::Script,
        binary_path: Some(path.to_path_buf()),
        app_id: None,
        icon: desktop_entry.and_then(|e| e.icon.clone()),
        desktop_file: desktop_entry.map(|e| e.path.clone()),
        categories: desktop_entry
            .map(|e| e.categories.clone())
            .unwrap_or_default(),
        generic_name: desktop_entry.and_then(|e| e.generic_name.clone()),
        comment: desktop_entry.and_then(|e| e.comment.clone()),
    })
}

fn capitalize_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use pipelink_common::types::{Pid, StartTime};

    use super::*;

    #[test]
    fn test_java_app() {
        let proc = ProcessInfo {
            pid: Pid(100),
            ppid: Some(Pid(1)),
            comm: "java".to_string(),
            exe: Some(PathBuf::from("/usr/bin/java")),
            cmdline: vec![
                "java".to_string(),
                "-jar".to_string(),
                "/opt/minecraft/minecraft.jar".to_string(),
            ],
            environ: std::collections::HashMap::default(),
            cgroup: None,
            start_time: Some(StartTime(1234)),
            cwd: None,
            root_dir: PathBuf::from("/"),
        };
        let db = DesktopDatabase::default();
        let unwrapped = build_java_app(&proc, &db);
        assert_eq!(unwrapped.app_name, "minecraft");
        assert_eq!(unwrapped.wrapper, WrapperKind::Java);
    }
}
