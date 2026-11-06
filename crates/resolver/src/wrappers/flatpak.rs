/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{fs, path::Path};

use pipelink_common::types::{AppId, Pid};

use crate::{
    desktop::DesktopDatabase,
    proc::ProcessInfo,
    wrappers::{UnwrappedApp, WrapperKind},
};

/// Inspects a process to determine if it is running inside or managed by Flatpak.
#[must_use]
pub fn inspect_flatpak(
    proc: &ProcessInfo,
    ancestry: &[ProcessInfo],
    desktop_db: &DesktopDatabase,
) -> Option<UnwrappedApp> {
    // 1. Check FLATPAK_ID in environment
    if let Some(flatpak_id) = proc.environ.get("FLATPAK_ID") {
        return Some(build_flatpak_app(proc.pid, flatpak_id, proc, desktop_db));
    }

    // 2. Check /.flatpak-info in container root
    if let Some(flatpak_id) = read_flatpak_info_id(&proc.root_dir) {
        return Some(build_flatpak_app(proc.pid, &flatpak_id, proc, desktop_db));
    }

    // Also check pid dir directly in case root mount is not directly traversable
    let pid_dir = proc.root_dir.parent().unwrap_or(Path::new(""));
    let direct_info = pid_dir.join(".flatpak-info");
    if let Some(flatpak_id) = read_flatpak_info_file(&direct_info) {
        return Some(build_flatpak_app(proc.pid, &flatpak_id, proc, desktop_db));
    }

    // 3. Check cgroup for flatpak pattern
    if let Some(cgroup) = &proc.cgroup
        && let Some(flatpak_id) = parse_cgroup_flatpak_id(cgroup)
    {
        return Some(build_flatpak_app(proc.pid, &flatpak_id, proc, desktop_db));
    }

    // 4. Check if process is `flatpak run ...` or `bwrap`
    if let Some(bin) = proc.binary_name()
        && (bin == "flatpak" || bin == "bwrap")
        && let Some(flatpak_id) = extract_flatpak_id_from_cmdline(&proc.cmdline)
    {
        return Some(build_flatpak_app(proc.pid, &flatpak_id, proc, desktop_db));
    }

    // 5. Check ancestry for Flatpak environment
    for ancestor in ancestry {
        if let Some(flatpak_id) = ancestor.environ.get("FLATPAK_ID") {
            return Some(build_flatpak_app(proc.pid, flatpak_id, proc, desktop_db));
        }
        if let Some(flatpak_id) = read_flatpak_info_id(&ancestor.root_dir) {
            return Some(build_flatpak_app(proc.pid, &flatpak_id, proc, desktop_db));
        }
    }

    None
}

/// Reads Flatpak App ID from `/.flatpak-info` inside the container root.
fn read_flatpak_info_id(root_dir: &Path) -> Option<String> {
    let info_path = root_dir.join(".flatpak-info");
    read_flatpak_info_file(&info_path)
}

/// Parses a `.flatpak-info` keyfile to extract `[Application] name=<app-id>`.
fn read_flatpak_info_file(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let mut in_app_section = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_app_section = line == "[Application]";
            continue;
        }

        if in_app_section
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == "name"
        {
            let id = v.trim().to_string();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }

    None
}

/// Parses cgroup strings like
/// `0::/user.slice/.../app-flatpak-org.mozilla.firefox-12345.scope`.
fn parse_cgroup_flatpak_id(cgroup: &str) -> Option<String> {
    for line in cgroup.lines() {
        if let Some(idx) = line.find("app-flatpak-") {
            let rest = &line[idx + "app-flatpak-".len()..];
            let id_part = rest.split('/').next()?.split(".scope").next()?;
            let candidate = if let Some((id, _)) = id_part.rsplit_once('-') {
                id
            } else {
                id_part
            };
            if candidate.contains('.') {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

/// Extracts flatpak application ID from command line like `flatpak run --branch=stable
/// com.spotify.Client`.
fn extract_flatpak_id_from_cmdline(cmdline: &[String]) -> Option<String> {
    for arg in cmdline.iter().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        if arg.contains('.')
            && arg
                .chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-')
        {
            return Some(arg.clone());
        }
    }
    None
}

/// Constructs the `UnwrappedApp` from the resolved Flatpak App ID.
fn build_flatpak_app(
    target_pid: Pid,
    flatpak_id: &str,
    proc: &ProcessInfo,
    desktop_db: &DesktopDatabase,
) -> UnwrappedApp {
    let flatpak_app_id = AppId::from(flatpak_id);
    let desktop_entry = desktop_db.find_by_flatpak_id(&flatpak_app_id);

    let display_name = desktop_entry.map(|e| e.name.clone()).or_else(|| {
        flatpak_id
            .rsplit('.')
            .next()
            .map(std::string::ToString::to_string)
    });

    let app_name = display_name.as_deref().map_or_else(
        || {
            flatpak_id
                .rsplit('.')
                .next()
                .unwrap_or(flatpak_id)
                .to_lowercase()
        },
        str::to_lowercase,
    );

    let icon = desktop_entry.and_then(|e| e.icon.clone());
    let desktop_file = desktop_entry.map(|e| e.path.clone());
    let categories = desktop_entry
        .map(|e| e.categories.clone())
        .unwrap_or_default();
    let generic_name = desktop_entry.and_then(|e| e.generic_name.clone());
    let comment = desktop_entry.and_then(|e| e.comment.clone());

    UnwrappedApp {
        target_pid,
        app_name,
        display_name,
        wrapper: WrapperKind::Flatpak,
        binary_path: proc.exe.clone(),
        app_id: Some(flatpak_app_id),
        icon,
        desktop_file,
        categories,
        generic_name,
        comment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cgroup() {
        let cgroup = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-org.mozilla.firefox-12345.scope";
        assert_eq!(
            parse_cgroup_flatpak_id(cgroup),
            Some("org.mozilla.firefox".to_string())
        );
    }

    #[test]
    fn test_extract_flatpak_cmdline() {
        let cmd = vec![
            "flatpak".to_string(),
            "run".to_string(),
            "--branch=stable".to_string(),
            "com.core447.StreamController".to_string(),
        ];
        assert_eq!(
            extract_flatpak_id_from_cmdline(&cmd),
            Some("com.core447.StreamController".to_string())
        );
    }
}
