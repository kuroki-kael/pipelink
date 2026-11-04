/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#![allow(
    clippy::case_sensitive_file_extension_comparisons,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use pipelink_common::app::WrapperKind;

pub mod electron;
pub mod flatpak;
pub mod script;
pub mod wine;

/// Intermediate result when unwrapping a wrapper or container layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwrappedApp {
    /// The actual target PID representing the main application
    pub target_pid: u32,
    /// Canonical short application name (e.g. "streamcontroller", "reaper", "discord")
    pub app_name: String,
    /// Clean display name if determined
    pub display_name: Option<String>,
    /// Type of wrapper detected
    pub wrapper: WrapperKind,
    /// Resolved binary path (host path, or Windows executable path)
    pub binary_path: Option<PathBuf>,
    /// Application ID if known (e.g. Flatpak ID or Steam App ID)
    pub app_id: Option<String>,
    /// Icon name or path
    pub icon: Option<String>,
    /// Desktop entry file path if matched
    pub desktop_file: Option<PathBuf>,
    /// Desktop categories
    pub categories: Vec<String>,
    /// Generic application name
    pub generic_name: Option<String>,
    /// Comment / description
    pub comment: Option<String>,
}
