/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Application metadata and wrapper representations.

use std::{fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::types::{AppId, Pid};

/// Classification of the wrapper or execution environment detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WrapperKind {
    /// Native binary directly executed without wrapper.
    #[default]
    None,
    /// Flatpak container sandbox.
    Flatpak,
    /// Wine Windows compatibility layer.
    Wine,
    /// Valve Proton (Steam Wine compatibility tool).
    Proton,
    /// Electron / Chromium multi-process runtime.
    Electron,
    /// Canonical Snap package sandbox.
    Snap,
    /// `AppImage` portable bundle.
    AppImage,
    /// Generic Bubblewrap container (e.g. Steam pressure-vessel).
    Bubblewrap,
    /// Script interpreter wrapper (Bash, Sh, Python, Perl, Ruby, Node).
    Script,
    /// Java Virtual Machine runtime.
    Java,
}

impl fmt::Display for WrapperKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "Native"),
            Self::Flatpak => write!(f, "Flatpak"),
            Self::Wine => write!(f, "Wine"),
            Self::Proton => write!(f, "Proton"),
            Self::Electron => write!(f, "Electron"),
            Self::Snap => write!(f, "Snap"),
            Self::AppImage => write!(f, "AppImage"),
            Self::Bubblewrap => write!(f, "Bubblewrap"),
            Self::Script => write!(f, "Script"),
            Self::Java => write!(f, "Java"),
        }
    }
}

/// Full description of an application resolved from a process ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Default)]
pub struct App {
    /// Original process ID queried (e.g. from `PipeWire`).
    pub pid: Pid,
    /// The target process ID after peeling away wrapper helpers (e.g. Electron
    /// `AudioService` -> main process).
    pub actual_pid: Pid,
    /// Short canonical application slug (e.g. "streamcontroller", "discord", "reaper").
    pub app_name: String,
    /// Human-friendly display name (e.g. "`StreamController`", "Discord", "REAPER").
    pub display_name: String,
    /// Wrapper type detected (Flatpak, Wine, Proton, Electron, Snap, `AppImage`, Script,
    /// None).
    pub wrapper: WrapperKind,
    /// Absolute path to executable binary (or Windows executable path for Wine).
    pub binary_path: Option<PathBuf>,
    /// Application ID if known (e.g. Flatpak ID "com.discordapp.Discord" or Steam App
    /// ID).
    pub app_id: Option<AppId>,
    /// Icon name or path from desktop file if found.
    pub icon: Option<String>,
    /// Path to matching `.desktop` entry file.
    pub desktop_file: Option<PathBuf>,
    /// `FreeDesktop` categories (e.g. `["Audio", "Network"]`).
    pub categories: Vec<String>,
    /// Generic name from desktop entry (e.g. "Music Player", "Web Browser").
    pub generic_name: Option<String>,
    /// Application description or comment.
    pub comment: Option<String>,
    /// Command line arguments of the resolved process.
    pub cmdline: Vec<String>,
    /// Working directory of the resolved process.
    pub cwd: Option<PathBuf>,
}
