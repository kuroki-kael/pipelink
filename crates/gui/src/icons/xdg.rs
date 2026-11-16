/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! XDG desktop icon discovery across SVG and PNG icon themes for installed and active
//! apps.

use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use iced::widget::svg::Handle;
use pipelink_common::events::InstalledAppInfo;

const SIZE_DIRS: &[&str] = &[
    "scalable",
    "64x64",
    "48x48",
    "128x128",
    "32x32",
    "96x96",
    "256x256",
    "512x512",
    "24x24",
    "16x16",
    "64x64@2",
    "48x48@2",
    "32x32@2",
    "128x128@2",
    "1024x1024",
];

const BREEZE_SIZES: &[&str] = &["scalable", "64", "48", "32", "24", "16"];
const MAX_PNG_FILE_BYTES: u64 = 2 * 1024 * 1024;
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Resolves `.svg` and `.png` application icons across configured and fallback XDG
/// icon themes, returning `svg::Handle` values ready for zero-I/O UI rendering.
pub(crate) fn resolve_app_icons(apps: &[InstalledAppInfo]) -> HashMap<String, Handle> {
    let dirs = ordered_icon_dirs();
    let index = build_icon_index(&dirs);
    let mut resolved = HashMap::new();

    for app in apps {
        let mut keys = Vec::new();
        if let Some(hint) = app.icon.as_deref() {
            keys.push(hint);
        }
        keys.push(app.id.as_ref());
        if let Some(bin) = app.exec_bin.as_deref() {
            keys.push(bin);
        }

        let Some(handle) = keys.iter().find_map(|key| load_icon_for_key(key, &index)) else {
            continue;
        };

        for key in keys {
            resolved
                .entry(key.to_ascii_lowercase())
                .or_insert_with(|| handle.clone());
        }
    }

    resolved
}

fn load_icon_for_key(key: &str, index: &HashMap<String, PathBuf>) -> Option<Handle> {
    let candidate = if Path::new(key).is_absolute() {
        PathBuf::from(key)
    } else {
        let stem = strip_image_ext(key).to_ascii_lowercase();
        index.get(&stem)?.clone()
    };

    load_icon_file(&candidate)
}

fn load_icon_file(path: &Path) -> Option<Handle> {
    let ext = path.extension()?.to_str()?;
    if ext.eq_ignore_ascii_case("svg") && path.is_file() {
        return Some(Handle::from_path(path));
    }
    if ext.eq_ignore_ascii_case("png")
        && let Ok(meta) = fs::metadata(path)
        && meta.is_file()
        && meta.len() <= MAX_PNG_FILE_BYTES
        && let Ok(bytes) = fs::read(path)
    {
        return Some(Handle::from_memory(wrap_png_as_svg(&bytes)));
    }
    None
}

/// Embeds a PNG byte buffer inside a tiny 64×64 SVG wrapper so `resvg` renders both
/// vector and raster desktop icons through `iced::widget::svg` cleanly offline.
fn wrap_png_as_svg(png_bytes: &[u8]) -> Vec<u8> {
    let encoded = encode_base64(png_bytes);
    let mut out = String::with_capacity(encoded.len() + 128);
    out.push_str(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><image width="64" height="64" preserveAspectRatio="xMidYMid meet" href="data:image/png;base64,"#,
    );
    out.push_str(&encoded);
    out.push_str(r#""/></svg>"#);
    out.into_bytes()
}

fn encode_base64(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = usize::from(chunk[0]);
        let b1 = usize::from(chunk.get(1).copied().unwrap_or(0));
        let b2 = usize::from(chunk.get(2).copied().unwrap_or(0));
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(char::from(BASE64_ALPHABET[(triple >> 18) & 0x3F]));
        out.push(char::from(BASE64_ALPHABET[(triple >> 12) & 0x3F]));
        out.push(if chunk.len() > 1 {
            char::from(BASE64_ALPHABET[(triple >> 6) & 0x3F])
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            char::from(BASE64_ALPHABET[triple & 0x3F])
        } else {
            '='
        });
    }
    out
}

fn build_icon_index(dirs: &[PathBuf]) -> HashMap<String, PathBuf> {
    let mut index = HashMap::new();

    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut svg_entries = Vec::new();
        let mut png_entries = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if ext.eq_ignore_ascii_case("svg") {
                svg_entries.push(path);
            } else if ext.eq_ignore_ascii_case("png") {
                png_entries.push(path);
            }
        }

        for path in svg_entries.into_iter().chain(png_entries) {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                index.entry(stem.to_ascii_lowercase()).or_insert(path);
            }
        }
    }

    index
}

fn ordered_icon_dirs() -> Vec<PathBuf> {
    let bases = data_bases();
    let themes = ordered_themes();
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();

    for theme in &themes {
        for base in &bases {
            let theme_dir = base.join("icons").join(theme);
            if !theme_dir.is_dir() {
                continue;
            }
            for size in SIZE_DIRS {
                push_unique_dir(&mut dirs, &mut seen, theme_dir.join(size).join("apps"));
            }
            for size in BREEZE_SIZES {
                push_unique_dir(&mut dirs, &mut seen, theme_dir.join("apps").join(size));
            }
            for cat in ["categories", "preferences", "devices", "status"] {
                push_unique_dir(&mut dirs, &mut seen, theme_dir.join("scalable").join(cat));
                push_unique_dir(&mut dirs, &mut seen, theme_dir.join("48x48").join(cat));
            }
        }
    }

    for base in &bases {
        push_unique_dir(&mut dirs, &mut seen, base.join("pixmaps"));
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        push_unique_dir(&mut dirs, &mut seen, home.join(".icons"));
    }

    for theme in &themes {
        for base in &bases {
            push_unique_dir(
                &mut dirs,
                &mut seen,
                base.join("icons").join(theme).join("symbolic/apps"),
            );
        }
    }

    dirs
}

fn push_unique_dir(out: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, dir: PathBuf) {
    if dir.is_dir() && seen.insert(dir.clone()) {
        out.push(dir);
    }
}

fn data_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Some(data_home) = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    {
        bases.push(data_home.join("flatpak/exports/share"));
        bases.push(data_home);
    }

    let data_dirs = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| {
        "/usr/local/share:/usr/share:/var/lib/flatpak/exports/share".to_string()
    });
    for base in data_dirs.split(':').filter(|s| !s.is_empty()) {
        bases.push(PathBuf::from(base));
    }
    bases.push(PathBuf::from("/var/lib/flatpak/exports/share"));
    bases
}

fn ordered_themes() -> Vec<String> {
    let mut themes = Vec::new();
    if let Some(active) = detect_active_icon_theme() {
        themes.push(active);
    }
    for fallback in [
        "hicolor",
        "breeze-dark",
        "breeze",
        "Papirus-Dark",
        "Papirus",
        "Adwaita",
    ] {
        if !themes.iter().any(|t| t == fallback) {
            themes.push(fallback.to_string());
        }
    }
    themes
}

fn detect_active_icon_theme() -> Option<String> {
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;

    if let Ok(kde) = fs::read_to_string(config_home.join("kdeglobals"))
        && let Some(theme) = parse_ini_key(&kde, "Icons", "Theme")
    {
        return Some(theme);
    }
    if let Ok(gtk) = fs::read_to_string(config_home.join("gtk-3.0/settings.ini"))
        && let Some(theme) = parse_ini_key(&gtk, "Settings", "gtk-icon-theme-name")
    {
        return Some(theme);
    }
    None
}

fn parse_ini_key(contents: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for raw in contents.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = &line[1..line.len() - 1] == section;
            continue;
        }
        if in_section
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            let value = v.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn strip_image_ext(key: &str) -> &str {
    let path = Path::new(key);
    if path.extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("png")
            || ext.eq_ignore_ascii_case("svg")
            || ext.eq_ignore_ascii_case("xpm")
    }) {
        path.file_stem().and_then(|s| s.to_str()).unwrap_or(key)
    } else {
        key
    }
}
