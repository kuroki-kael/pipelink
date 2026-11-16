/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! WaveLink-inspired `iced` graphical routing frontend for `PipeLink`.

mod icons;
mod state;
mod subscription;
mod theme;
mod views;

#[cfg(test)]
mod tests;

use state::App;

/// Starts the graphical interface window on the calling thread.
///
/// # Errors
/// Returns an [`iced::Error`] if creating the window or graphics backend fails.
pub fn run() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title("PipeLink")
        .theme(|_: &App| theme::app_theme())
        .window(iced::window::Settings {
            icon: icons::window_icon(),
            ..Default::default()
        })
        .window_size((1180.0, 720.0))
        .centered()
        .subscription(|_: &App| subscription::daemon_events())
        .run()
}
