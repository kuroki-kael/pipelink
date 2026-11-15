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
    graph::{app_node::AppNode, channel::Channel, link::StereoLink, mix::Mix},
    types::{ChannelName, LinkId, MixName, NodeId},
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
        /// App to route
        app: AppNode,
        /// Destination channel
        channel: Channel,
    },

    SetMixRoute {
        mix: Mix,
        // TODO!!! implement audio device structs
    },

    /// clears `target.object` metadata
    ClearNodeRoute {
        node_id: NodeId,
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
