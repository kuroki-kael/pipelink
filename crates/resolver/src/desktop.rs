/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use pipelink_common::types::AppId;
use tracing::debug;

use crate::tokenize_cmdline;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub id: AppId,
    pub path: PathBuf,
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub exec_bin: Option<String>,
    pub startup_wm_class: Option<String>,
    pub categories: Vec<String>,
    pub x_flatpak: Option<AppId>,
    pub no_display: bool,
}

impl DesktopEntry {
    pub fn new(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        let id = AppId::from(path.file_stem()?.to_string_lossy().into_owned());

        let mut name = None;
        let mut generic_name = None;
        let mut comment = None;
        let mut icon = None;
        let mut exec = None;
        let mut startup_wm_class = None;
        let mut categories = Vec::new();
        let mut x_flatpak = None;
        let mut no_display = false;
        let mut entry_type = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line == "[Desktop Entry]" {
                continue;
            }

            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                let val = v.trim();

                match key {
                    "Type" => entry_type = Some(val.to_string()),
                    "Name" if name.is_none() => name = Some(val.to_string()),
                    "GenericName" if generic_name.is_none() => generic_name = Some(val.to_string()),
                    "Comment" if comment.is_none() => comment = Some(val.to_string()),
                    "Icon" => icon = Some(val.to_string()),
                    "Exec" => exec = Some(val.to_string()),
                    "StartupWMClass" => startup_wm_class = Some(val.to_string()),
                    "Categories" => {
                        categories = val.split(';').map(ToString::to_string).collect();
                    },
                    "X-Flatpak" => x_flatpak = Some(AppId::from(val)),
                    "NoDisplay" => no_display = val.eq_ignore_ascii_case("true"),
                    _ => {},
                }
            }
        }

        if let Some(t) = entry_type
            && t != "Application"
        {
            return None;
        }

        let name = name.unwrap_or_else(|| id.0.clone());

        let exec_bin = exec.as_ref().and_then(|cmd| Self::extract_exec_binary(cmd));

        Some(Self {
            id,
            path: path.to_path_buf(),
            name,
            generic_name,
            comment,
            icon,
            exec,
            exec_bin,
            startup_wm_class,
            categories,
            x_flatpak,
            no_display,
        })
    }

    fn extract_exec_binary(cmd: &str) -> Option<String> {
        let tokens = tokenize_cmdline(cmd);
        let mut iter = tokens.iter();
        let first = iter.next()?;

        let cmd = if first == "env" {
            let mut actual = None;
            for p in iter {
                if !p.contains('=') {
                    actual = Some(p);
                    break;
                }
            }
            actual?
        } else {
            first
        };

        Path::new(cmd)
            .file_name()
            .and_then(|s| s.to_str())
            .map(ToString::to_string)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DesktopDatabase {
    pub entries: Vec<DesktopEntry>,
    by_id: HashMap<AppId, usize>,
    by_flatpak_id: HashMap<AppId, usize>,
    by_wm_class: HashMap<String, usize>,
    by_exec_bin: HashMap<String, usize>,
    by_name: HashMap<String, usize>,
    dirs: Vec<PathBuf>,
    dir_mtimes: HashMap<PathBuf, SystemTime>,
}

/// Normalizes an application id for lookups: lower-cased and without a
/// trailing `.desktop` suffix.
fn normalize_app_id(id: &AppId) -> AppId {
    let lower = id.0.to_lowercase();
    AppId::from(lower.strip_suffix(".desktop").unwrap_or(&lower))
}

impl DesktopDatabase {
    pub fn load_standard(custom_dirs: &[PathBuf]) -> Self {
        let mut dirs = Vec::new();

        let standard_fallbacks = [
            "/usr/local/share",
            "/usr/share",
            "/var/lib/flatpak/exports/share",
            "/var/lib/snapd/desktop",
        ];

        let mut add_if_valid = |base: PathBuf| {
            let app_dir = if base.ends_with("applications") {
                base
            } else {
                base.join("applications")
            };

            if app_dir.is_dir() && !dirs.contains(&app_dir) {
                dirs.push(app_dir);
            }
        };

        if let Some(xdg_home) = env::var_os("XDG_DATA_HOME") {
            add_if_valid(PathBuf::from(xdg_home));
        } else if let Some(home) = env::var_os("HOME") {
            add_if_valid(PathBuf::from(&home).join(".local/share/applications"));
            add_if_valid(
                PathBuf::from(home).join(".local/share/flatpak/exports/share/applications"),
            );
        }

        if let Some(data_dirs) = env::var_os("XDG_DATA_DIRS") {
            for path in env::split_paths(&data_dirs) {
                add_if_valid(path);
            }
        }

        for path in standard_fallbacks {
            add_if_valid(PathBuf::from(path));
        }

        for path in custom_dirs {
            add_if_valid(path.clone());
        }

        Self::load_from_dirs(&dirs)
    }

    fn load_from_dirs(dirs: &[PathBuf]) -> Self {
        let mut db = DesktopDatabase {
            dirs: dirs.to_vec(),
            ..Default::default()
        };

        let mut seen_ids = HashSet::new();

        for dir in dirs {
            if let Ok(meta) = fs::metadata(dir)
                && let Ok(mtime) = meta.modified()
            {
                db.dir_mtimes.insert(dir.clone(), mtime);
            }

            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                    continue;
                }

                let id = match path.file_stem().and_then(|i| i.to_str()) {
                    Some(id) => AppId::from(id),
                    None => continue,
                };

                if seen_ids.contains(&id) {
                    continue;
                }

                if let Some(desktop_entry) = DesktopEntry::new(&path) {
                    seen_ids.insert(id);
                    db.add_entry(desktop_entry);
                }
            }
        }

        debug!(
            "Indexed {} entries across {} dirs",
            db.entries.len(),
            db.dirs.len()
        );

        db
    }

    fn add_entry(&mut self, entry: DesktopEntry) {
        let idx = self.entries.len();

        self.by_id.entry(normalize_app_id(&entry.id)).or_insert(idx);

        if let Some(flatpak_id) = &entry.x_flatpak {
            self.by_flatpak_id
                .entry(normalize_app_id(flatpak_id))
                .or_insert(idx);
        }

        if let Some(wm_class) = &entry.startup_wm_class {
            self.by_wm_class
                .entry(wm_class.to_lowercase())
                .or_insert(idx);
        }

        if let Some(bin) = &entry.exec_bin {
            let key = bin.to_lowercase();

            let is_direct_match =
                entry.id.0.to_lowercase().ends_with(&key) || entry.name.eq_ignore_ascii_case(&key);

            match self.by_exec_bin.get(&key) {
                None => {
                    self.by_exec_bin.insert(key, idx);
                },
                Some(&existing_idx) => {
                    let existing_entry = &self.entries[existing_idx];
                    let existing_is_direct = existing_entry.id.0.to_lowercase().ends_with(&key)
                        || existing_entry.name.eq_ignore_ascii_case(&key);

                    if is_direct_match && !existing_is_direct {
                        self.by_exec_bin.insert(key, idx);
                    }
                },
            }
        }

        let name_lower = entry.name.to_lowercase();
        self.by_name.entry(name_lower).or_insert(idx);

        self.entries.push(entry);
    }

    pub fn find_by_flatpak_id(&self, flatpak_id: &AppId) -> Option<&DesktopEntry> {
        let key = normalize_app_id(flatpak_id);
        self.by_flatpak_id
            .get(&key)
            .or_else(|| self.by_id.get(&key))
            .map(|&idx| &self.entries[idx])
    }

    pub fn find_by_id(&self, id: &AppId) -> Option<&DesktopEntry> {
        let key = normalize_app_id(id);
        self.by_id.get(&key).map(|&idx| &self.entries[idx])
    }

    pub fn find_by_binary(&self, bin: &str) -> Option<&DesktopEntry> {
        let key = bin.to_lowercase();
        self.by_exec_bin.get(&key).map(|&idx| &self.entries[idx])
    }

    pub fn find_by_wm_class(&self, wm_class: &str) -> Option<&DesktopEntry> {
        let key = wm_class.to_lowercase();
        self.by_wm_class.get(&key).map(|&idx| &self.entries[idx])
    }

    pub fn find_by_name(&self, name: &str) -> Option<&DesktopEntry> {
        let key = name.to_lowercase();
        self.by_name.get(&key).map(|&idx| &self.entries[idx])
    }

    pub fn find_best_match(&self, hint: &str) -> Option<&DesktopEntry> {
        let clean = hint.trim();
        if clean.is_empty() {
            return None;
        }

        let app_id = AppId::from(clean);

        if let Some(entry) = self.find_by_flatpak_id(&app_id) {
            return Some(entry);
        }
        if let Some(entry) = self.find_by_id(&app_id) {
            return Some(entry);
        }
        if let Some(entry) = self.find_by_name(clean) {
            return Some(entry);
        }
        if let Some(entry) = self.find_by_wm_class(clean) {
            return Some(entry);
        }
        if let Some(entry) = self.find_by_binary(clean) {
            return Some(entry);
        }

        let hint_lower = clean.to_lowercase();
        for entry in &self.entries {
            let id_lower = entry.id.0.to_lowercase();
            if id_lower.ends_with(&format!(".{hint_lower}")) || id_lower == hint_lower {
                return Some(entry);
            }
        }

        None
    }

    pub fn reload_if_changed(&mut self) -> bool {
        let mut changed = false;
        for dir in &self.dirs {
            if let Ok(meta) = fs::metadata(dir)
                && let Ok(mtime) = meta.modified()
            {
                if let Some(&last_mtime) = self.dir_mtimes.get(dir) {
                    if mtime > last_mtime {
                        changed = true;
                        break;
                    }
                } else {
                    changed = true;
                    break;
                }
            }
        }

        if changed {
            *self = Self::load_from_dirs(&self.dirs);
        }

        changed
    }
}
