/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Settings and diagnostics view implemented on [`App`].

use std::{env, path::PathBuf};

use iced::{
    Element, Length,
    widget::{column, container, text},
};

use crate::{
    state::{App, Message},
    theme,
};

impl App {
    /// Renders daemon status, indexed app summary, and resolved storage paths.
    pub(super) fn view_settings(&self) -> Element<'_, Message> {
        let status_text = if self.connected {
            format!(
                "Connected to daemon ({} channels, {} mixes, {} app routes)",
                self.snapshot.channels.len(),
                self.snapshot.mixes.len(),
                self.snapshot.app_routes.len()
            )
        } else {
            "Daemon not connected".to_string()
        };

        let status_card = container(
            column![
                text("Daemon Status").size(16).color(theme::TEXT_PRIMARY),
                text(status_text).size(13).color(theme::TEXT_MUTED),
                text(format!(
                    "Installed .desktop apps indexed: {}",
                    self.installed_apps.len()
                ))
                .size(13)
                .color(theme::TEXT_MUTED),
            ]
            .spacing(6),
        )
        .padding(16)
        .width(Length::Fill)
        .style(theme::card_style);

        let config_path = resolve_xdg_file(
            "PIPELINK_CONFIG",
            "XDG_CONFIG_HOME",
            ".config",
            "config.json",
        );
        let state_path = resolve_xdg_file(
            "PIPELINK_STATE",
            "XDG_STATE_HOME",
            ".local/state",
            "state.json",
        );

        let about_card = container(
            column![
                text("About PipeLink").size(16).color(theme::TEXT_PRIMARY),
                text(format!("Version {}", env!("CARGO_PKG_VERSION")))
                    .size(13)
                    .color(theme::TEXT_MUTED),
                text(format!("Config: {config_path}"))
                    .size(12)
                    .color(theme::TEXT_MUTED),
                text(format!("State: {state_path}"))
                    .size(12)
                    .color(theme::TEXT_MUTED),
            ]
            .spacing(6),
        )
        .padding(16)
        .width(Length::Fill)
        .style(theme::card_style);

        column![status_card, about_card]
            .spacing(16)
            .width(Length::Fill)
            .into()
    }
}

/// Resolves the active `config.json` or `state.json` path on disk.
fn resolve_xdg_file(
    override_var: &str,
    xdg_var: &str,
    home_fallback_rel: &str,
    file_name: &str,
) -> String {
    if let Some(path) = env::var_os(override_var).filter(|v| !v.is_empty()) {
        return PathBuf::from(path).display().to_string();
    }

    if let Some(xdg) = env::var_os(xdg_var).filter(|v| !v.is_empty()) {
        return PathBuf::from(xdg)
            .join("pipelink")
            .join(file_name)
            .display()
            .to_string();
    }

    if let Some(home) = env::var_os("HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(home)
            .join(home_fallback_rel)
            .join("pipelink")
            .join(file_name)
            .display()
            .to_string();
    }

    format!("pipelink/{file_name}")
}
