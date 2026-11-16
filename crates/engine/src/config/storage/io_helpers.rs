/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Atomic JSON serialization, corrupt-file backup, and XDG path resolution helpers.

use std::{
    env,
    ffi::OsStr,
    fs::{self, DirBuilder, OpenOptions},
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process,
};

use serde::{Serialize, de::DeserializeOwned};
use tracing::warn;

const APP_DIR: &str = "pipelink";

pub(super) fn resolve_file_path(
    override_env: &str,
    xdg_env: &str,
    home_fallback_rel: &str,
    file_name: &str,
) -> Option<PathBuf> {
    if let Some(path) = env::var_os(override_env).filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(path));
    }
    let base_dir = env::var_os(xdg_env)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|home| PathBuf::from(home).join(home_fallback_rel))
        })?;
    Some(base_dir.join(APP_DIR).join(file_name))
}

pub(super) fn load_json_or_default<T: DeserializeOwned + Default>(path: &Path) -> T {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return T::default(),
        Err(err) => {
            warn!("Failed to read '{}': {err}; using defaults", path.display());
            return T::default();
        },
    };

    match serde_json::from_slice::<T>(&bytes) {
        Ok(parsed) => parsed,
        Err(err) => {
            let backup = backup_path(path);
            if let Err(rename_err) = fs::rename(path, &backup) {
                warn!(
                    "Malformed JSON in '{}' ({err}); also failed to move backup '{}': {rename_err}",
                    path.display(),
                    backup.display()
                );
            } else {
                warn!(
                    "Malformed JSON in '{}' ({err}); moved corrupt file to '{}' and reset to defaults",
                    path.display(),
                    backup.display()
                );
            }
            T::default()
        },
    }
}

pub(super) fn backup_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(OsStr::new(".bak"));
    PathBuf::from(os)
}

pub(super) fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }

    let payload = serde_json::to_vec_pretty(value)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let tmp = temp_path_for(path);
    let write_res = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(&payload)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();

    if write_res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    write_res
}

fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pipelink");
    let tmp_name = format!(".{file_name}.tmp.{}", process::id());
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(tmp_name),
        _ => PathBuf::from(tmp_name),
    }
}
