/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Audio routing topology models and configuration graph.

use pipelink_common::types::{AppId, ChannelName, MixName, NodeName};
use serde::{Deserialize, Serialize};

/// A virtual Sink an app or microphone gets routed to.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub struct AudioChannel {
    /// Channel name.
    pub name: ChannelName,
    /// App identifiers routed to this channel.
    #[serde(default)]
    pub apps: Vec<AppId>,
    /// Optional physical input source.
    #[serde(default)]
    pub source: Option<NodeName>,
}

/// A consumer of one or more channels.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub struct AudioMix {
    /// Mix name.
    pub name: MixName,
    /// Channels routed to mix.
    #[serde(default)]
    pub channels: Vec<ChannelName>,
}

/// In-memory and serialized representation of the audio routing graph.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConfigGraph {
    /// Physical Sink name the active mix routes to.
    #[serde(default)]
    pub output: Option<NodeName>,
    /// Name of mix that goes to output.
    #[serde(default)]
    pub active_mix: Option<MixName>,
    /// All defined channels.
    #[serde(default)]
    pub channels: Vec<AudioChannel>,
    /// All defined mixes.
    #[serde(default)]
    pub mixes: Vec<AudioMix>,
}

impl ConfigGraph {
    /// Creates a new empty [`ConfigGraph`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
