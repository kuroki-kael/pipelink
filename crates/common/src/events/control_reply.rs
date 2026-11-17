/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::{Deserialize, Serialize};

use crate::{
    graph::output::preferred_output_label,
    types::{AppDisplay, AppId, ChannelName, MixName, NodeName},
};

/// Summary of an active resolved application in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveAppInfo {
    /// Human-friendly display name (e.g. `"Discord"`).
    pub display_name: AppDisplay,
    /// Canonical application slug (e.g. `"discord"`).
    pub app_name: String,
    /// Assigned channel name, if routed.
    pub channel: Option<ChannelName>,
    /// Number of active `PipeWire` stream nodes belonging to this app.
    pub stream_count: usize,
}

/// Summary of an available `PipeWire` audio output sink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputDeviceInfo {
    /// `PipeWire` `node.name`.
    pub name: NodeName,
    /// Human-readable device name (`node.description`), if set.
    pub description: Option<String>,
    /// Short device nickname (`node.nick`), if set.
    pub nick: Option<String>,
    /// `true` if backed by a hardware device (`device.id` present).
    pub is_hardware: bool,
    /// `true` if currently selected as the active mix output.
    pub active: bool,
}

impl OutputDeviceInfo {
    /// Preferred human-readable label (`description`, `nick`, or `name`).
    #[must_use]
    pub fn display_name(&self) -> &str {
        preferred_output_label(
            self.description.as_deref(),
            self.nick.as_deref(),
            &self.name,
        )
    }
}

/// State of a single `(channel, mix)` route cell inside a graph snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixRouteInfo {
    /// Source channel name.
    pub channel: ChannelName,
    /// Target mix name.
    pub mix: MixName,
    /// Whether audio is currently routed from `channel` into `mix`.
    pub active: bool,
    /// Gain percentage (`0.0` = silence, `100.0` = unity).
    pub gain_percent: f32,
    /// Whether this route is muted.
    pub muted: bool,
}

/// Summary of an installed `.desktop` application discovered by the resolver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledAppInfo {
    /// Desktop file identifier (e.g. `"firefox"` or `"com.discordapp.Discord"`).
    pub id: AppId,
    /// Human-readable application display name.
    pub name: String,
    /// Optional XDG icon name or path from the `.desktop` entry.
    pub icon: Option<String>,
    /// Executable binary name extracted from `Exec=`, if available.
    pub exec_bin: Option<String>,
}

/// Master volume and mute state for a channel's `PipeWire` null-sink.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelVolumeInfo {
    /// Channel name.
    pub channel: ChannelName,
    /// Master volume percentage (`100.0` = unity).
    pub volume_percent: f32,
    /// Whether the channel's null-sink is muted.
    pub muted: bool,
}

/// Complete snapshot of the routing topology, mixer cells, outputs, and app routes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphSnapshot {
    /// Configured channel names.
    pub channels: Vec<ChannelName>,
    /// Per-channel null-sink master volume and mute states.
    #[serde(default)]
    pub channel_volumes: Vec<ChannelVolumeInfo>,
    /// Configured mix names.
    pub mixes: Vec<MixName>,
    /// Globally active mix driving the audio output, if any.
    pub active_mix: Option<MixName>,
    /// Current `WirePlumber` configured default audio sink, if known.
    pub default_sink: Option<NodeName>,
    /// Full channel-to-mix matrix cells.
    pub routes: Vec<MixRouteInfo>,
    /// Available `PipeWire` audio output sinks.
    pub outputs: Vec<OutputDeviceInfo>,
    /// Currently running resolved applications.
    pub apps: Vec<ActiveAppInfo>,
    /// Configured application-to-channel routing rules.
    pub app_routes: Vec<(AppDisplay, ChannelName)>,
}

/// Answer to a control event that asks for state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ControlReply {
    /// The names of all channels.
    Channels { names: Vec<ChannelName> },
    /// The names of all mixes.
    Mixes { names: Vec<MixName> },
    /// Currently active resolved applications and their routing status.
    Apps { apps: Vec<ActiveAppInfo> },
    /// The apps and the channel each one is routed to.
    AppChannels {
        routes: Vec<(AppDisplay, ChannelName)>,
    },
    /// The mixes and each channel routed to each.
    MixChannels {
        channels: Vec<(MixName, Vec<ChannelName>)>,
    },
    /// The active mix, if one is selected.
    ActiveMix { mix: Option<MixName> },
    /// All available audio output sinks.
    Outputs { outputs: Vec<OutputDeviceInfo> },
    /// Full routing graph and mixer state snapshot.
    GraphSnapshot(GraphSnapshot),
    /// Installed desktop applications known to the resolver.
    InstalledApps { apps: Vec<InstalledAppInfo> },
}
