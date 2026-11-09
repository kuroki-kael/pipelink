/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use serde::{Deserialize, Serialize};

use crate::types::{AppDisplay, ChannelName, MixName};

/// Answer to a control event that asks for state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ControlReply {
    /// The names of all channels.
    Channels { names: Vec<ChannelName> },
    /// The names of all mixes.
    Mixes { names: Vec<MixName> },
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
