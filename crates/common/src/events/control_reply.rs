/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::{Deserialize, Serialize};

use crate::types::{AppDisplay, ChannelName, MixName};

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
}
