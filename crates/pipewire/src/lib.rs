/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

mod error;
mod events;
mod factory;
mod loop_state;
mod module_handle;
mod pw_loop;

pub use error::PipewireError;
pub use events::PipewireEvent;
pub(crate) use loop_state::LoopState;
use pipelink_common::{
    graph::{channel::Channel, link::StereoLink, mix::Mix},
    types::{ChannelName, LinkId, MixName, NodeId, NodeName, NodeSerial},
};
pub use pw_loop::PipewireService;

/// Commands sent from the engine to the `PipeWire` background loop.
#[derive(Debug, Clone)]
pub enum Command {
    CreateLink {
        stereo_link: StereoLink,
    },

    DestroyLink {
        link_id: LinkId,
    },

    /// sets `target.object` metadata
    SetAppRoute {
        /// Stream node ID to route
        node_id: NodeId,
        /// Serial of the destination channel node
        target_serial: NodeSerial,
        /// Application label for logging
        label: String,
    },

    SetMixRoute {
        mix: Mix,
        // TODO!!! implement audio device structs
    },

    /// clears `target.object` metadata
    ClearNodeRoute {
        node_id: NodeId,
    },

    /// Sets `default.configured.audio.sink` in `PipeWire`'s `default` metadata.
    SetDefaultSink {
        node_name: NodeName,
    },

    /// Sets master linear volume and mute flags on a channel's `PipeWire` null-sink.
    SetChannelNodeProps {
        node_id: NodeId,
        linear_volume: f32,
        muted: bool,
    },

    CreateChannel {
        name: ChannelName,
    },

    CreateMix {
        name: MixName,
    },

    LoadInput {
        name: String,
        nick: String,
    },

    /// issues round-trip on pipewire
    CoreSync,

    DestroyNode {
        node_id: NodeId,
    },

    DeleteChannel {
        channel: Channel,
    },

    DeleteMix {
        mix: Mix,
    },
}
