/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::{Deserialize, Serialize};

use crate::types::{ChannelName, MixName};

/// Events sent from controllers (IPC, CLI, GUI, MIDI, Web) to request state changes and
/// queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ControlEvent {
    /// Sets channel gain for a specific mix.
    SetChannelGain {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
        /// Gain value in decibels.
        db_value: f32,
    },
    /// Sets mute status of a channel for a specific mix.
    SetChannelMute {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
        /// Whether the channel should be muted.
        muted: bool,
    },
    /// Requests creation of a new audio channel.
    CreateChannel {
        /// Channel name to create.
        name: ChannelName,
    },
    /// Requests removal of an audio channel.
    RemoveChannel {
        /// Channel name to remove.
        name: ChannelName,
    },
    /// Requests creation of a new audio mix.
    CreateMix {
        /// Mix name to create.
        name: MixName,
    },
    /// Requests removal of an audio mix.
    RemoveMix {
        /// Mix name to remove.
        name: MixName,
    },
    /// Selects the global active mix layer.
    SelectGlobalActiveLayer {
        /// Mix name to activate globally.
        mix: MixName,
    },
    /// Selects the active layer for a specific channel.
    SelectChannelActiveLayer {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
    },
    /// Adds a channel to a mix.
    AddChannelMix {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
    },
    /// Removes a channel from a mix.
    RemoveChannelMix {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
    },
    /// Requests the names of all channels.
    ListChannels,
    /// Requests the names of all mixes.
    ListMixes,
    /// Requests the apps and the channel each one is routed to.
    ListAppChannels,
    /// Requests the mixes and the channels routed to each.
    ListMixChannels,
    /// Requests the globally active mix.
    GetActiveMix,
}
