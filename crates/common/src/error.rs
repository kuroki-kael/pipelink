/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Pipewire error: {0}")]
    Pipewire(String),

    #[error("Pipewire channel error: {0}")]
    PipewireChannel(String),

    #[error("JACK error: {0}")]
    Jack(String),

    #[error("Internal channel closed")]
    ChannelClosed,
}
