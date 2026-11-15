/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use thiserror::Error;

use crate::types::{ChannelName, MixName};

/// Errors raised by the audio engine while coordinating the routing graph.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExecutorError {
    #[error("Pipewire error: {0}")]
    Pipewire(String),

    #[error("JACK error: {0}")]
    Jack(String),

    #[error("Internal channel closed")]
    ChannelClosed,

    #[error("Channel already exists: {0}")]
    ChannelExists(ChannelName),

    #[error("Mix already exists: {0}")]
    MixExists(MixName),
}
