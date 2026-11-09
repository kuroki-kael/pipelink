/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::{Deserialize, Serialize};

use crate::types::{ChannelName, MixName};

/// Events broadcast by the audio engine to notify listeners of state changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateEvent {
    /// Gain changed for a channel within a mix.
    GainChanged {
        /// Affected channel name.
        channel: ChannelName,
        /// Affected mix name.
        mix: MixName,
        /// Linear gain multiplier.
        gain_linear: f32,
        /// Gain value in decibels.
        db_value: f32,
    },
    /// Mute state changed for a channel within a mix.
    MuteChanged {
        /// Affected channel name.
        channel: ChannelName,
        /// Affected mix name.
        mix: MixName,
        /// Whether the channel is muted.
        muted: bool,
    },
    /// Global active layer switched to a new mix.
    GlobalLayerSwitched {
        /// New active mix name.
        mix: MixName,
    },
    /// Active layer switched for a channel.
    ChannelLayerSwitched {
        /// Affected channel name.
        channel: ChannelName,
        /// Affected mix name.
        mix: MixName,
    },
    /// New channel created.
    ChannelAdded {
        /// Channel name.
        channel: ChannelName,
    },
    /// Channel deleted.
    ChannelRemoved {
        /// Channel name.
        channel: ChannelName,
    },
    /// New mix created.
    MixAdded {
        /// Mix name.
        mix: MixName,
    },
    /// Mix deleted.
    MixRemoved {
        /// Mix name.
        mix: MixName,
    },
    /// Channel associated with a mix.
    MixChannelAdded {
        /// Channel name.
        channel: ChannelName,
        /// Mix name.
        mix: MixName,
    },
    /// Channel dissociated from a mix.
    MixChannelRemoved {
        /// Channel name.
        channel: ChannelName,
        /// Mix name.
        mix: MixName,
    },
}
