/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink_common::graph::{app_node::AppNode, channel::Channel, link::StereoLink, mix::Mix};

mod config;
pub mod executor;

pub use config::topology::{AudioChannel, AudioMix, ConfigGraph};

/// In-memory state of the audio routing graph.
///
/// Owned by the [`Executor`](executor::Executor). The mix half of the graph
/// (`mixes`, `active_mix`, `links`) is not wired up yet.
#[derive(Clone, Debug, Default)]
pub struct AudioGraph {
    pub channels: Vec<Channel>,
    pub mixes: Vec<Mix>,
    pub active_mix: Option<Mix>,
    pub links: Vec<StereoLink>,
    pub apps: Vec<AppNode>,
}

impl AudioGraph {
    /// Creates an empty audio graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
