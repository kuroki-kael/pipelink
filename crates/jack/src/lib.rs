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
    CreateChannel {
        ports: ChannelPorts,
        gains: Vec<f32>,
    },
    RemoveChannel {
        idx: usize,
        gains: Vec<f32>,
    },
    CreateMix {
        ports: MixPorts,
        gains: Vec<f32>,
    },
    RemoveMix {
        idx: usize,
        gains: Vec<f32>,
    },
    SetGains(Vec<f32>),
}

/// Resources retired by the audio thread for off-thread cleanup and deallocation.
#[derive(Debug)]
pub(crate) enum RetiredResource {
    Channel(ChannelPorts),
    Mix(MixPorts),
    Gains(Vec<f32>),
}
