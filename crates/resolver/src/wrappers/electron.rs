/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::path::Path;

use crate::{
    desktop::DesktopDatabase,
    proc::ProcessInfo,
    wrappers::{UnwrappedApp, WrapperKind},
};

/// Inspects a process to determine if it is an Electron or Chromium application or
/// helper.
#[must_use]
pub fn inspect_electron(
    proc: &ProcessInfo,
    ancestry: &[ProcessInfo],
    desktop_db: &DesktopDatabase,
) -> Option<UnwrappedApp> {
    if !is_electron_or_chromium(proc) && !ancestry.iter().any(is_electron_or_chromium) {
        return None;
    }

    // Find the main process:
    // The main process usually does NOT have `--type=`.
    let main_proc = ancestry
        .iter()
        .rev()
        .find(|info| is_electron_or_chromium(info) && !is_sub_process(info))
        .or_else(|| {
            if is_sub_process(proc) {
                ancestry.first()
            } else {
                Some(proc)
            }
        })
        .unwrap_or(proc);

    let target_pid = main_proc.pid;

    let mut asar_name = None;
    let mut user_data_name = None;
    let mut app_url_or_path = None;

    let mut all_procs = vec![proc];
    all_procs.extend(ancestry.iter());

    for p in all_procs {
        if asar_name.is_none()
            && let Some(name) = extract_asar_app_name(&p.cmdline)
        {
            asar_name = Some(name);
        }
        if user_data_name.is_none()
            && let Some(dir) = extract_user_data_dir_name(&p.cmdline)
        {
            user_data_name = Some(dir);
        }
        if app_url_or_path.is_none()
            && let Some(app) = extract_app_flag(&p.cmdline)
        {
            app_url_or_path = Some(app);
        }
    }

    let mut candidate_slug = None;
    let mut candidate_display = None;

    if let Some(bin) = main_proc.binary_name()
        && !is_generic_electron_binary(bin)
    {
        candidate_slug = Some(bin.to_lowercase());
        candidate_display = Some(bin.to_string());
    }

    if candidate_slug.is_none()
        && let Some(asar) = asar_name
    {
        candidate_slug = Some(asar.to_lowercase());
        candidate_display = Some(asar);
    }

    if let Some(user_data) = user_data_name {
        if candidate_display.is_none() {
            candidate_display = Some(user_data.clone());
        }
        if candidate_slug.is_none() {
            candidate_slug = Some(user_data.to_lowercase().replace(' ', "-"));
        }
    }

    if candidate_slug.is_none()
        && let Some(app) = app_url_or_path
    {
        candidate_slug = Some(app.clone());
        candidate_display = Some(app);
    }

    let app_name = candidate_slug.unwrap_or_else(|| {
        main_proc
            .binary_name()
            .unwrap_or(&main_proc.comm)
            .to_string()
    });

    let desktop_entry = desktop_db.find_best_match(&app_name).or_else(|| {
        candidate_display
            .as_deref()
            .and_then(|disp| desktop_db.find_best_match(disp))
    });

    let display_name = desktop_entry
        .map(|e| e.name.clone())
        .or(candidate_display)
        .or_else(|| Some(app_name.clone()));

    let icon = desktop_entry.and_then(|e| e.icon.clone());
    let desktop_file = desktop_entry.map(|e| e.path.clone());
    let categories = desktop_entry
        .map(|e| e.categories.clone())
        .unwrap_or_default();
    let generic_name = desktop_entry.and_then(|e| e.generic_name.clone());
    let comment = desktop_entry.and_then(|e| e.comment.clone());

    let binary_path = main_proc.exe.clone().or_else(|| proc.exe.clone());

    Some(UnwrappedApp {
        target_pid,
        app_name,
        display_name,
        wrapper: WrapperKind::Electron,
        binary_path,
        app_id: desktop_entry.map(|e| e.id.clone()),
        icon,
        desktop_file,
        categories,
        generic_name,
        comment,
    })
}

fn is_generic_electron_binary(bin: &str) -> bool {
    let lower = bin.to_lowercase();
    lower == "electron"
        || lower.starts_with("electron")
        || lower == "chrome"
        || lower == "chromium"
        || lower == "google-chrome"
        || lower == "brave"
}

fn is_electron_or_chromium(proc: &ProcessInfo) -> bool {
    if let Some(bin) = proc.binary_name()
        && is_generic_electron_binary(bin)
    {
        return true;
    }

    for arg in &proc.cmdline {
        if arg.starts_with("--type=")
            || arg.starts_with("--app-path=")
            || arg.starts_with("--user-data-dir=")
            || arg.ends_with(".asar")
            || arg.contains("/resources/app")
        {
            return true;
        }
    }

    proc.comm == "electron"
        || proc.comm == "AudioService"
        || proc.comm == "Isolated Web Co"
        || proc.comm == "zygote"
}

fn is_sub_process(proc: &ProcessInfo) -> bool {
    proc.cmdline.iter().any(|arg| arg.starts_with("--type="))
        || proc.comm == "AudioService"
        || proc.comm == "zygote"
        || proc.comm == "Isolated Web Co"
}

fn extract_asar_app_name(cmdline: &[String]) -> Option<String> {
    for arg in cmdline {
        let path_str = if let Some(stripped) = arg.strip_prefix("--app-path=") {
            stripped
        } else if arg.ends_with(".asar") || arg.contains(".asar/") {
            arg.as_str()
        } else {
            continue;
        };

        let path = Path::new(path_str);
        let mut curr = if path.is_file() || path_str.ends_with(".asar") {
            path.parent()
        } else {
            Some(path)
        };

        while let Some(dir) = curr {
            let name = dir.file_name()?.to_str()?;
            if name != "resources" && name != "app" && name != "dist" && name != "build" {
                return Some(name.to_string());
            }
            curr = dir.parent();
        }
    }
    None
}

fn extract_user_data_dir_name(cmdline: &[String]) -> Option<String> {
    for arg in cmdline {
        if let Some(dir_str) = arg.strip_prefix("--user-data-dir=") {
            let path = Path::new(dir_str);
            if let Some(folder_name) = path.file_name().and_then(|n| n.to_str())
                && !folder_name.is_empty()
                && folder_name != "Default"
                && folder_name != "Profile"
            {
                return Some(folder_name.to_string());
            }
        }
    }
    None
}

fn extract_app_flag(cmdline: &[String]) -> Option<String> {
    for arg in cmdline {
        if let Some(app) = arg.strip_prefix("--app=") {
            return Some(app.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_asar_app_name() {
        let cmd = vec![
            "/usr/lib/electron42/electron".to_string(),
            "/usr/lib/pear-desktop/app.asar".to_string(),
        ];
        assert_eq!(
            extract_asar_app_name(&cmd),
            Some("pear-desktop".to_string())
        );

        let cmd2 = vec![
            "/opt/discord/Discord".to_string(),
            "/opt/discord/resources/app.asar".to_string(),
        ];
        assert_eq!(extract_asar_app_name(&cmd2), Some("discord".to_string()));
    }

    #[test]
    fn test_extract_user_data_dir() {
        let cmd = vec!["--user-data-dir=/home/user/.config/YouTube Music".to_string()];
        assert_eq!(
            extract_user_data_dir_name(&cmd),
            Some("YouTube Music".to_string())
        );
    }
}
