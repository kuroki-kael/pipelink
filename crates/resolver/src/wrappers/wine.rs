/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{
    fs,
    path::{Path, PathBuf},
};

use pipelink_common::types::{AppId, SteamAppId};

use crate::{
    desktop::DesktopDatabase,
    proc::ProcessInfo,
    wrappers::{UnwrappedApp, WrapperKind},
};

const WINE_HELPERS: &[&str] = &[
    "wine",
    "wine64",
    "wineserver",
    "wineboot",
    "winecfg",
    "winedbg",
    "conhost",
    "explorer",
    "rpcss",
    "services",
    "svchost",
    "rundll32",
    "start",
    "dxvk-cache-builder",
];

/// Inspects a process to determine if it is running under Wine or Valve Proton.
#[must_use]
pub fn inspect_wine(
    proc: &ProcessInfo,
    ancestry: &[ProcessInfo],
    desktop_db: &DesktopDatabase,
) -> Option<UnwrappedApp> {
    let mut is_proton = false;
    let mut steam_app_id = None;

    let is_wine = check_is_wine_process(proc, &mut is_proton, &mut steam_app_id)
        || ancestry
            .iter()
            .any(|a| check_is_wine_process(a, &mut is_proton, &mut steam_app_id));

    if !is_wine {
        return None;
    }

    let wrapper = if is_proton {
        WrapperKind::Proton
    } else {
        WrapperKind::Wine
    };

    let mut target_exe = extract_windows_exe(&proc.cmdline);
    let mut target_pid = proc.pid;

    if target_exe.is_none() {
        for ancestor in ancestry {
            if let Some(exe) = extract_windows_exe(&ancestor.cmdline) {
                target_exe = Some(exe);
                target_pid = ancestor.pid;
                break;
            }
        }
    }

    if target_exe.is_none()
        && proc.comm.to_lowercase().ends_with(".exe")
        && !is_wine_helper(&proc.comm)
    {
        target_exe = Some(proc.comm.clone());
    }

    let mut steam_game_name = None;
    if let Some(app_id) = steam_app_id
        && let Some(name) = resolve_steam_app_name(app_id)
    {
        steam_game_name = Some(name);
    }

    let app_name = if let Some(steam_name) = &steam_game_name {
        steam_name.clone()
    } else if let Some(exe) = &target_exe {
        clean_exe_name(exe)
    } else {
        "wine-application".to_string()
    };

    let display_name =
        steam_game_name.or_else(|| target_exe.as_ref().map(|exe| format_exe_display_name(exe)));

    let app_id: Option<AppId> = steam_app_id.map(SteamAppId::to_app_id);

    let desktop_entry = desktop_db
        .find_best_match(&app_name)
        .or_else(|| app_id.as_ref().and_then(|id| desktop_db.find_by_id(id)));

    let icon = desktop_entry.and_then(|e| e.icon.clone());
    let desktop_file = desktop_entry.map(|e| e.path.clone());
    let categories =
        desktop_entry.map_or_else(|| vec!["Wine".to_string()], |e| e.categories.clone());
    let generic_name = desktop_entry.and_then(|e| e.generic_name.clone());
    let comment = desktop_entry.and_then(|e| e.comment.clone());

    let binary_path = target_exe.map(PathBuf::from).or_else(|| proc.exe.clone());

    Some(UnwrappedApp {
        target_pid,
        app_name,
        display_name,
        wrapper,
        binary_path,
        app_id,
        icon,
        desktop_file,
        categories,
        generic_name,
        comment,
    })
}

/// Parses a Steam application id from an environment variable value.
///
/// Steam exposes the id as a plain number; values that do not parse are
/// discarded instead of being propagated into the app id namespace.
fn parse_steam_app_id(raw: &str) -> Option<SteamAppId> {
    raw.trim().parse::<SteamAppId>().ok()
}

fn check_is_wine_process(
    proc: &ProcessInfo,
    is_proton: &mut bool,
    steam_app_id: &mut Option<SteamAppId>,
) -> bool {
    let mut detected = false;

    if let Some(raw) = proc.environ.get("STEAM_COMPAT_APP_ID") {
        *is_proton = true;
        detected = true;
        if steam_app_id.is_none() {
            *steam_app_id = parse_steam_app_id(raw);
        }
    } else if let Some(raw) = proc.environ.get("SteamAppId") {
        detected = true;
        if steam_app_id.is_none() {
            *steam_app_id = parse_steam_app_id(raw);
        }
    }

    if proc.environ.contains_key("PROTON_VERSION")
        || proc.environ.contains_key("STEAM_COMPAT_DATA_PATH")
    {
        *is_proton = true;
        detected = true;
    }

    if proc.environ.contains_key("WINEPREFIX") || proc.environ.contains_key("WINEDEBUG") {
        detected = true;
    }

    if let Some(exe) = &proc.exe {
        let exe_str = exe.to_string_lossy().to_lowercase();
        if exe_str.contains("/proton") || exe_str.contains("/proton/") {
            *is_proton = true;
            detected = true;
        }
        if let Some(bin) = proc.binary_name() {
            let bin_lower = bin.to_lowercase();
            if bin_lower.starts_with("wine") || bin_lower == "wineserver" {
                detected = true;
            }
        }
    }

    if proc.cmdline.iter().any(|arg| {
        let lower = arg.to_lowercase();
        lower.ends_with(".exe") || lower.contains("c:\\") || lower.contains("z:\\")
    }) {
        detected = true;
    }

    detected
}

fn is_wine_helper(name: &str) -> bool {
    let lower = name.to_lowercase();
    let clean = lower.trim_end_matches(".exe");
    WINE_HELPERS
        .iter()
        .any(|&h| h.trim_end_matches(".exe") == clean)
}

fn extract_windows_exe(cmdline: &[String]) -> Option<String> {
    for arg in cmdline {
        let lower = arg.to_lowercase();
        if lower.ends_with(".exe") || lower.ends_with(".bat") || lower.ends_with(".msi") {
            let file_name = if let Some(idx) = arg.rfind('\\') {
                &arg[idx + 1..]
            } else if let Some(idx) = arg.rfind('/') {
                &arg[idx + 1..]
            } else {
                arg.as_str()
            };

            if !is_wine_helper(file_name) {
                return Some(file_name.to_string());
            }
        }
    }
    None
}

fn clean_exe_name(exe: &str) -> String {
    let base = if let Some(idx) = exe.rfind('\\') {
        &exe[idx + 1..]
    } else if let Some(idx) = exe.rfind('/') {
        &exe[idx + 1..]
    } else {
        exe
    };

    base.strip_suffix(".exe")
        .or_else(|| base.strip_suffix(".EXE"))
        .unwrap_or(base)
        .to_lowercase()
}

fn format_exe_display_name(exe: &str) -> String {
    let clean = clean_exe_name(exe);
    if clean
        .chars()
        .all(|c| c.is_lowercase() || c.is_numeric() || c == '_' || c == '-')
    {
        let mut chars = clean.chars();
        match chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    } else {
        clean
    }
}

fn resolve_steam_app_name(app_id: SteamAppId) -> Option<String> {
    let manifest_name = format!("appmanifest_{app_id}.acf");
    let mut candidate_dirs = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let home_path = Path::new(&home);
        candidate_dirs.push(home_path.join(".local/share/Steam/steamapps"));
        candidate_dirs.push(home_path.join(".steam/steam/steamapps"));
        candidate_dirs.push(home_path.join(".steam/root/steamapps"));
    }

    for dir in candidate_dirs {
        let manifest_path = dir.join(&manifest_name);
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("\"name\"") {
                    let parts: Vec<&str> = trimmed.split('"').collect();
                    if parts.len() >= 4 {
                        let name = parts[3].trim();
                        if !name.is_empty() {
                            return Some(name.to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_windows_exe() {
        let cmdline = vec![
            "/usr/bin/wine64-preloader".to_string(),
            "C:\\Program Files\\REAPER\\reaper.exe".to_string(),
            "--wait".to_string(),
        ];
        assert_eq!(
            extract_windows_exe(&cmdline),
            Some("reaper.exe".to_string())
        );
    }

    #[test]
    fn test_clean_exe_name() {
        assert_eq!(clean_exe_name("reaper.exe"), "reaper");
        assert_eq!(clean_exe_name("CYBERPUNK2077.EXE"), "cyberpunk2077");
    }

    #[test]
    fn test_parse_steam_app_id() {
        assert_eq!(parse_steam_app_id("620"), Some(SteamAppId(620)));
        assert_eq!(parse_steam_app_id(" 620 "), Some(SteamAppId(620)));
        assert_eq!(parse_steam_app_id(""), None);
        assert_eq!(parse_steam_app_id("not-a-number"), None);
    }

    #[test]
    fn test_steam_app_id_to_app_id() {
        assert_eq!(SteamAppId(620).to_app_id(), AppId::from("steam_620"));
    }
}
