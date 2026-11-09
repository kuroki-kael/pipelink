/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, PoisonError, RwLock},
};

use pipelink_common::{
    app::{App, WrapperKind},
    types::{Pid, StartTime},
};
use tracing::debug;

use crate::{
    desktop::DesktopDatabase,
    proc::{ProcessInfo, is_process_alive_in, read_process_ancestry, read_process_info_from_proc},
    wrappers::{
        UnwrappedApp, electron::inspect_electron, flatpak::inspect_flatpak,
        script::inspect_script_or_package, wine::inspect_wine,
    },
};

/// Configuration options for the [`ProcessResolver`].
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Maximum ancestry search depth when traversing process parents.
    pub max_ancestry_depth: usize,
    /// Additional custom directories to scan for `.desktop` files.
    pub custom_desktop_dirs: Vec<PathBuf>,
    /// Base `/proc` filesystem directory path (defaults to `"/proc"`).
    pub proc_dir: PathBuf,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            max_ancestry_depth: 16,
            custom_desktop_dirs: Vec::new(),
            proc_dir: PathBuf::from("/proc"),
        }
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    app: Arc<App>,
    start_time: Option<StartTime>,
}

/// High-performance process inspector and resolution service.
///
/// Traverses process hierarchies in `/proc`, analyzes environment variables,
/// command line flags, and `.desktop` files to map a Linux `pid` to an [`App`].
#[derive(Debug)]
pub struct ProcessResolver {
    config: ResolverConfig,
    desktop_db: Arc<RwLock<DesktopDatabase>>,
    cache: RwLock<HashMap<Pid, CacheEntry>>,
}

impl Default for ProcessResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessResolver {
    /// Creates a new resolver with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ResolverConfig::default())
    }

    /// Creates a new [`ProcessResolver`] with custom configuration.
    #[must_use]
    pub fn with_config(config: ResolverConfig) -> Self {
        let db = DesktopDatabase::load_standard(&config.custom_desktop_dirs);
        Self {
            config,
            desktop_db: Arc::new(RwLock::new(db)),
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Resolves an application by its Linux process ID.
    ///
    /// If the process was previously resolved and is still alive with the same
    /// start time, this returns a cheap [`Arc`] clone from memory cache in $O(1)$.
    #[must_use]
    pub fn resolve(&self, pid: impl Into<Pid>) -> Option<Arc<App>> {
        let pid = pid.into();
        // 1. Fast Cache Check
        if let Ok(cache) = self.cache.read()
            && let Some(entry) = cache.get(&pid)
            && is_process_alive_in(&self.config.proc_dir, pid)
            && let Ok(info) = read_process_info_from_proc(&self.config.proc_dir, pid)
            && (info.start_time == entry.start_time || entry.start_time.is_none())
        {
            return Some(Arc::clone(&entry.app));
        }

        // 2. Uncached full resolution
        let app = self.resolve_uncached(pid)?;
        let app_arc = Arc::new(app);

        // 3. Store in cache
        if let Ok(mut cache) = self.cache.write() {
            let start_time = read_process_info_from_proc(&self.config.proc_dir, pid)
                .ok()
                .and_then(|info| info.start_time);

            cache.insert(
                pid,
                CacheEntry {
                    app: Arc::clone(&app_arc),
                    start_time,
                },
            );
        }

        Some(app_arc)
    }

    /// Resolves the application directly from `/proc` without consulting or updating the
    /// cache.
    #[must_use]
    pub fn resolve_uncached(&self, pid: impl Into<Pid>) -> Option<App> {
        let pid = pid.into();
        debug!("Resolving process {pid}");
        let proc = read_process_info_from_proc(&self.config.proc_dir, pid).ok()?;
        let ancestry =
            read_process_ancestry(&self.config.proc_dir, pid, self.config.max_ancestry_depth);

        let unwrapped = self.resolve_with_db(&proc, &ancestry);
        Some(Self::build_resolved(pid, &proc, unwrapped))
    }

    fn resolve_with_db(&self, proc: &ProcessInfo, ancestry: &[ProcessInfo]) -> UnwrappedApp {
        let db = self
            .desktop_db
            .read()
            .unwrap_or_else(PoisonError::into_inner);

        // 1. Flatpak unwrapping
        if let Some(unwrapped) = inspect_flatpak(proc, ancestry, &db) {
            debug!(
                "Resolved PID {} via Flatpak unwrapper: {}",
                proc.pid, unwrapped.app_name
            );
            return unwrapped;
        }

        // 2. Wine / Proton unwrapping
        if let Some(unwrapped) = inspect_wine(proc, ancestry, &db) {
            debug!(
                "Resolved PID {} via Wine/Proton unwrapper: {}",
                proc.pid, unwrapped.app_name
            );
            return unwrapped;
        }

        // 3. Electron / Chromium unwrapping
        if let Some(unwrapped) = inspect_electron(proc, ancestry, &db) {
            debug!(
                "Resolved PID {} via Electron unwrapper: {}",
                proc.pid, unwrapped.app_name
            );
            return unwrapped;
        }

        // 4. Script / Snap / AppImage / Java unwrapping
        if let Some(unwrapped) = inspect_script_or_package(proc, ancestry, &db) {
            debug!(
                "Resolved PID {} via Script/Package unwrapper: {}",
                proc.pid, unwrapped.app_name
            );
            return unwrapped;
        }

        // 5. Native application (direct binary execution)
        let unwrapped = Self::resolve_native(proc, ancestry, &db);
        debug!(
            "Resolved PID {} as native application: {}",
            proc.pid, unwrapped.app_name
        );
        unwrapped
    }

    /// Fallback resolution for native processes that are not wrapped.
    fn resolve_native(
        proc: &ProcessInfo,
        ancestry: &[ProcessInfo],
        desktop_db: &DesktopDatabase,
    ) -> UnwrappedApp {
        // Handle browser content processes (e.g. Firefox / Zen browser content processes)
        let target_proc = if proc.cmdline.iter().any(|arg| arg == "-contentproc") {
            ancestry
                .iter()
                .find(|p| !p.cmdline.iter().any(|arg| arg == "-contentproc"))
                .unwrap_or(proc)
        } else {
            proc
        };

        let raw_name = target_proc
            .binary_name()
            .unwrap_or(&target_proc.comm)
            .to_string();

        let desktop_entry = desktop_db.find_best_match(&raw_name);

        let display_name = desktop_entry.map_or_else(|| capitalize(&raw_name), |e| e.name.clone());

        let app_name = raw_name.to_lowercase();
        let icon = desktop_entry.and_then(|e| e.icon.clone());
        let desktop_file = desktop_entry.map(|e| e.path.clone());
        let categories = desktop_entry
            .map(|e| e.categories.clone())
            .unwrap_or_default();
        let generic_name = desktop_entry.and_then(|e| e.generic_name.clone());
        let comment = desktop_entry.and_then(|e| e.comment.clone());

        UnwrappedApp {
            target_pid: target_proc.pid,
            app_name,
            display_name: Some(display_name),
            wrapper: WrapperKind::None,
            binary_path: target_proc.exe.clone(),
            app_id: desktop_entry.map(|e| e.id.clone()),
            icon,
            desktop_file,
            categories,
            generic_name,
            comment,
        }
    }

    fn build_resolved(original_pid: Pid, proc: &ProcessInfo, unwrapped: UnwrappedApp) -> App {
        let display_name = unwrapped
            .display_name
            .unwrap_or_else(|| capitalize(&unwrapped.app_name))
            .into();

        App {
            pid: original_pid,
            actual_pid: unwrapped.target_pid,
            app_name: unwrapped.app_name,
            display_name,
            wrapper: unwrapped.wrapper,
            binary_path: unwrapped.binary_path,
            app_id: unwrapped.app_id,
            icon: unwrapped.icon,
            desktop_file: unwrapped.desktop_file,
            categories: unwrapped.categories,
            generic_name: unwrapped.generic_name,
            comment: unwrapped.comment,
            cmdline: proc.cmdline.clone(),
            cwd: proc.cwd.clone(),
        }
    }

    /// Removes cached information for a specific process ID.
    pub fn invalidate(&self, pid: impl Into<Pid>) {
        let pid = pid.into().0;
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(&pid);
        }
    }

    /// Clears dead processes from the cache whose PID no longer exists in `/proc`.
    pub fn cleanup_dead_processes(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.retain(|&pid, _| is_process_alive_in(&self.config.proc_dir, pid));
        }
    }

    /// Clears the entire process resolution cache.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Checks if monitored desktop directories changed on disk and reloads if so.
    pub fn reload_desktop_database_if_changed(&self) -> bool {
        if let Ok(mut db) = self.desktop_db.write() {
            db.reload_if_changed()
        } else {
            false
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
