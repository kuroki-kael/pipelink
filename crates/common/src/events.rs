/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Domain events for audio routing control and state notifications.

use serde::{Deserialize, Serialize};

/// Events sent from controllers (IPC, CLI, GUI, MIDI, Web) to request state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlEvent {
    /// Sets channel gain for a specific mix.
    SetChannelGain {
        /// Target channel name.
        channel: String,
        /// Target mix name.
        mix: String,
        /// Gain value in decibels.
        db_value: f32,
    },
    /// Sets mute status of a channel for a specific mix.
    SetChannelMute {
        /// Target channel name.
        channel: String,
        /// Target mix name.
        mix: String,
        /// Whether the channel should be muted.
        muted: bool,
    },
    /// Requests creation of a new audio channel.
    CreateChannel {
        /// Channel name to create.
        name: String,
    },
    /// Requests removal of an audio channel.
    RemoveChannel {
        /// Channel name to remove.
        name: String,
    },
    /// Requests creation of a new audio mix.
    CreateMix {
        /// Mix name to create.
        name: String,
    },
    /// Requests removal of an audio mix.
    RemoveMix {
        /// Mix name to remove.
        name: String,
    },
    /// Selects the global active mix layer.
    SelectGlobalActiveLayer {
        /// Mix name to activate globally.
        mix: String,
    },
    /// Selects the active layer for a specific channel.
    SelectChannelActiveLayer {
        /// Target channel name.
        channel: String,
        /// Target mix name.
        mix: String,
    },
    /// Adds a channel to a mix.
    AddChannelMix {
        /// Target channel name.
        channel: String,
        /// Target mix name.
        mix: String,
    },
    /// Removes a channel from a mix.
    RemoveChannelMix {
        /// Target channel name.
        channel: String,
        /// Target mix name.
        mix: String,
    },
}

/// Events broadcast by the audio engine to notify listeners of state changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateChangeEvent {
    /// Gain changed for a channel within a mix.
    GainChanged {
        /// Affected channel name.
        channel: String,
        /// Affected mix name.
        mix: String,
        /// Linear gain multiplier.
        gain_linear: f32,
        /// Gain value in decibels.
        db_value: f32,
    },
    /// Mute state changed for a channel within a mix.
    MuteChanged {
        /// Affected channel name.
        channel: String,
        /// Affected mix name.
        mix: String,
        /// Whether the channel is muted.
        muted: bool,
    },
    /// Global active layer switched to a new mix.
    GlobalLayerSwitched {
        /// New active mix name.
        mix: String,
    },
    /// Active layer switched for a channel.
    ChannelLayerSwitched {
        /// Affected channel name.
        channel: String,
        /// New active mix name.
        mix: String,
    },
    /// New channel created.
    ChannelAdded {
        /// Channel name.
        channel: String,
    },
    /// Channel deleted.
    ChannelRemoved {
        /// Channel name.
        channel: String,
    },
    /// New mix created.
    MixAdded {
        /// Mix name.
        mix: String,
    },
    /// Mix deleted.
    MixRemoved {
        /// Mix name.
        mix: String,
    },
    /// Channel associated with a mix.
    ChannelMixAdded {
        /// Channel name.
        channel: String,
        /// Mix name.
        mix: String,
    },
    /// Channel dissociated from a mix.
    ChannelMixRemoved {
        /// Channel name.
        channel: String,
        /// Mix name.
        mix: String,
    },
}
