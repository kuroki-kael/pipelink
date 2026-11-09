/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

mod cli;
mod daemon;
mod ipc;

#[cfg(feature = "gui")]
mod gui;

#[tokio::main]
async fn main() {
    cli::run().await;
}
