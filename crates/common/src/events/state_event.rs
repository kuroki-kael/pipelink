/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::{Deserialize, Serialize};

use crate::types::{AppDisplay, ChannelName, MixName, NodeName};

/// Events broadcast by the audio engine to notify listeners of state changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum StateEvent {
    /// Gain changed for a channel within a mix.
    GainChanged {
        /// Affected channel name.
        channel: ChannelName,
        /// Affected mix name.
        mix: MixName,
        /// Linear gain multiplier.
        gain_linear: f32,
        /// Gain percentage (`0.0` = silence, `100.0` = unity gain).
        percent: f32,
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
    /// Channel-to-mix route, gain, or mute update failed.
    RouteOperationFailed {
        /// Affected channel name.
        channel: ChannelName,
        /// Affected mix name.
        mix: MixName,
        /// Reason the update failed.
        error: String,
    },
    /// Application routed to a channel.
    AppRouted {
        /// Application display name.
        app: AppDisplay,
        /// Destination channel name.
        channel: ChannelName,
    },
    /// Application routing rule removed.
    AppUnrouted {
        /// Application display name.
        app: AppDisplay,
    },
    /// Application routing update failed.
    AppRoutingFailed {
        /// Application display name.
        app: AppDisplay,
        /// Reason the routing operation failed.
        error: String,
    },
    /// Active mix output sink changed (either manually or via failover).
    OutputChanged {
        /// Newly selected output `node.name`.
        output: NodeName,
    },
    /// Selecting the output sink failed.
    OutputOperationFailed {
        /// Reason the output update failed.
        error: String,
    },
    /// Selecting a mix as the global active layer failed.
    LayerSelectionFailed {
        /// Mix name that could not be activated.
        mix: MixName,
        /// Reason the activation failed.
        error: String,
    },
    /// `WirePlumber` default audio sink was updated.
    DefaultSinkChanged {
        /// Newly configured default sink `node.name`.
        sink: NodeName,
    },
    /// Updating `WirePlumber`'s default audio sink failed.
    DefaultSinkOperationFailed {
        /// Reason the default sink update failed.
        error: String,
    },
    /// Invoking any change for a channel failed.
    ChannelOperationFailed {
        /// Affected channel name.
        channel: ChannelName,
        /// Reason the operation failed.
        error: String,
    },
    /// Invoking any change for a mix failed.
    MixOperationFailed {
        /// Affected mix name.
        mix: MixName,
        /// Reason the operation failed.
        error: String,
    },
}
