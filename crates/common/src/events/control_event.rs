/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::{Deserialize, Serialize};

use crate::types::{AppDisplay, ChannelName, MixName};

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
        /// Gain percentage (`0.0` = silence, `100.0` = unity gain).
        percent: f32,
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
    /// Sets master gain percentage on a channel's `PipeWire` null-sink (`0.0`..=`200.0`).
    SetChannelMasterVolume {
        /// Target channel name.
        channel: ChannelName,
        /// Master volume percentage (`100.0` = unity).
        percent: f32,
    },
    /// Sets master mute state on a channel's `PipeWire` null-sink.
    SetChannelMasterMute {
        /// Target channel name.
        channel: ChannelName,
        /// Whether the null-sink should be muted.
        muted: bool,
    },
    /// Requests creation of a new audio channel.
    CreateChannel {
        /// Channel name to create.
        name: ChannelName,
    },
    /// Requests renaming an existing channel while preserving its mix and app routes.
    RenameChannel {
        /// Existing channel name.
        old_name: ChannelName,
        /// Desired new channel name.
        new_name: ChannelName,
    },
    /// Reorders a channel to a target display index.
    MoveChannel {
        /// Channel name to move.
        channel: ChannelName,
        /// Zero-based destination row index.
        to_index: usize,
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
    /// Requests renaming an existing mix while preserving channel routes and gains.
    RenameMix {
        /// Existing mix name.
        old_name: MixName,
        /// Desired new mix name.
        new_name: MixName,
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
    /// Routes an application (by display name) into a channel.
    RouteApp {
        /// Application display name.
        app: AppDisplay,
        /// Destination channel name.
        channel: ChannelName,
    },
    /// Removes the channel routing rule for an application.
    UnrouteApp {
        /// Application display name.
        app: AppDisplay,
    },
    /// Requests the names of all channels.
    ListChannels,
    /// Requests the names of all mixes.
    ListMixes,
    /// Requests currently active resolved applications and their assigned channel.
    ListApps,
    /// Requests the apps and the channel each one is routed to.
    ListAppChannels,
    /// Requests the mixes and the channels routed to each.
    ListMixChannels,
    /// Requests the globally active mix.
    GetActiveMix,
    /// Requests all available audio output sinks.
    ListOutputs,
    /// Selects the audio output sink for the active mix.
    SetOutput {
        /// Output `node.name`, description, or nick.
        output: String,
    },
    /// Sets `WirePlumber`'s default audio sink (`default.configured.audio.sink`).
    SetDefaultSink {
        /// Channel name or output `node.name`, description, or nick.
        sink: String,
    },
    /// Requests a full snapshot of the routing graph, mixer levels, outputs, and apps.
    GetGraphSnapshot,
    /// Requests all installed desktop applications known to the resolver.
    ListInstalledApps,
    /// Subscribes to the continuous stream of daemon state events.
    SubscribeState,
}
