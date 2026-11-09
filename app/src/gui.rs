/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The graphical interface of the daemon.

/// Starts the graphical interface.
#[expect(
    clippy::unused_async,
    reason = "the interface awaits the daemon as soon as it is written"
)]
pub async fn run() {
    // Nothing to show yet, the cli reports the state of the daemon.
}
