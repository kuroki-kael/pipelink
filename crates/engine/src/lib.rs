/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink_common::graph::{app_node::AppNode, channel::Channel, link::StereoLink, mix::Mix};

mod config;
pub mod executor;

pub use config::topology::{AudioChannel, AudioMix, ConfigGraph};

#[derive(Clone, Debug, Default)]
pub struct AudioGraph {
    pub channels: Vec<Channel>,
    pub mixes: Vec<Mix>,
    pub active_mix: Option<Mix>,
    pub links: Vec<StereoLink>,
    pub apps: Vec<AppNode>,
}

impl AudioGraph {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
