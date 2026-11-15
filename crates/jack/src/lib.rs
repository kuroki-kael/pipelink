/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Real-time JACK audio mixer client and port management.

pub mod client;
pub mod error;
pub mod ports;
mod process;

pub use client::JackClient;
pub use error::JackError;

use crate::ports::{ChannelPorts, MixPorts};

/// Commands sent from [`JackClient`] to the real-time [`process::JackProcess`].
#[derive(Debug)]
pub(crate) enum Command {
    CreateChannel(ChannelPorts),
    RemoveChannel(usize),
    CreateMix(MixPorts),
    RemoveMix(usize),
    SetRoute {
        channel_idx: usize,
        mix_idx: usize,
        gain: f32,
    },
}

/// Ports retired by the audio thread for off-thread unregistration.
#[derive(Debug)]
pub(crate) enum RetiredResource {
    Channel(ChannelPorts),
    Mix(MixPorts),
}
