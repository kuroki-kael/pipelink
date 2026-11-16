/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Listing of installed `.desktop` applications exposed for UI app pickers.

use std::{collections::HashSet, sync::PoisonError};

use pipelink_common::events::InstalledAppInfo;

use crate::service::ProcessResolver;

impl ProcessResolver {
    /// Returns visible installed `.desktop` applications sorted alphabetically by name.
    #[must_use]
    pub fn installed_apps(&self) -> Vec<InstalledAppInfo> {
        let db = self
            .desktop_db
            .read()
            .unwrap_or_else(PoisonError::into_inner);

        let mut seen_names = HashSet::new();
        let mut apps: Vec<InstalledAppInfo> = db
            .entries
            .iter()
            .filter(|entry| !entry.no_display)
            .filter(|entry| seen_names.insert(entry.name.to_lowercase()))
            .map(|entry| InstalledAppInfo {
                id: entry.id.clone(),
                name: entry.name.clone(),
                icon: entry.icon.clone(),
                exec_bin: entry.exec_bin.clone(),
            })
            .collect();

        apps.sort_by_key(|a| a.name.to_lowercase());
        apps
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::service::{ProcessResolver, ResolverConfig};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let dir = env::temp_dir().join(format!("{prefix}-{nanos}"));
            let _ = fs::create_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_installed_apps_filters_nodisplay_and_sorts_alphabetically() {
        let dir = TempDir::new("pipelink-desktop-list");
        let apps_dir = dir.0.join("applications");
        let _ = fs::create_dir_all(&apps_dir);

        let _ = fs::write(
            apps_dir.join("zebra.desktop"),
            "[Desktop Entry]\nType=Application\nName=Zebra Player\nIcon=zebra\nExec=/usr/bin/zebra %U\n",
        );
        let _ = fs::write(
            apps_dir.join("alpha.desktop"),
            "[Desktop Entry]\nType=Application\nName=Alpha Chat\nIcon=alpha\nExec=alpha\n",
        );
        let _ = fs::write(
            apps_dir.join("hidden.desktop"),
            "[Desktop Entry]\nType=Application\nName=Hidden Helper\nNoDisplay=true\nExec=helper\n",
        );

        let resolver = ProcessResolver::with_config(ResolverConfig {
            custom_desktop_dirs: vec![apps_dir],
            ..Default::default()
        });

        let installed = resolver.installed_apps();
        assert!(installed.iter().all(|app| app.name != "Hidden Helper"));

        let pos_alpha = installed.iter().position(|app| app.name == "Alpha Chat");
        let pos_zebra = installed.iter().position(|app| app.name == "Zebra Player");
        assert!(pos_alpha.is_some());
        assert!(pos_zebra.is_some());
        assert!(pos_alpha < pos_zebra);
    }
}
