/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Errors raised by the JACK audio mixer client.

use pipelink_common::types::{ChannelName, MixName};
use thiserror::Error;

/// Errors raised while opening, configuring, or shutting down [`crate::JackClient`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JackError {
    /// Underlying error reported by the `jack` crate.
    #[error("JACK error: {0}")]
    Jack(#[from] jack::Error),

    /// Command channel to the real-time JACK process handler was closed.
    #[error("JACK command channel closed")]
    ChannelClosed,

    /// Command ringbuffer to the real-time JACK process handler is full.
    #[error("JACK command queue full")]
    ChannelFull,

    /// Attempted to register a channel that already exists in the JACK client.
    #[error("JACK channel already exists: {0}")]
    ChannelExists(ChannelName),

    /// Referenced a channel that is not registered in the JACK client.
    #[error("JACK channel not found: {0}")]
    ChannelNotFound(ChannelName),

    /// Attempted to register a mix that already exists in the JACK client.
    #[error("JACK mix already exists: {0}")]
    MixExists(MixName),

    /// Referenced a mix that is not registered in the JACK client.
    #[error("JACK mix not found: {0}")]
    MixNotFound(MixName),
}
