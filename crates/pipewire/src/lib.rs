/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

pub mod error;
pub mod events;
mod factory;
mod loop_state;
mod module_handle;
mod pw_loop;

pub use pw_loop::PipewireService;

pub(crate) use loop_state::LoopState;
use pipelink_common::graph::{
    app_node::AppNode,
    channel::Channel,
    link::{Link, StereoLink},
    mix::Mix,
};

#[derive(Debug, Clone)]
pub enum PipewireEvent {
    Ready,

    Disconnected {
        reason: String,
    },

    NodeAppeared {
        /// pipewire global id
        id: u32,
        /// `node.name` prop
        name: String,
        /// `media.class` prop (e.g. "Audio/Sink", "Audio/Source")
        media_class: String,
        /// Gets resolved by the process resolver
        app: Option<String>,
        /// `application.process.id` required for the resolver
        pid: Option<u32>,
        /// used for link generation
        serial: Option<u64>,
        /// `node.nick` prop
        nick: Option<String>,
    },

    NodeRemoved {
        id: u32,
    },

    LinkAppeared {
        /// Pipewire link id
        id: u32,
        /// Output node id
        out_node: u32,
        /// Input node id
        in_node: u32,
        /// Output port id
        out_port: u32,
        /// Input port id
        in_port: u32,
    },

    LinkRemoved {
        id: u32,
    },

    PortAppeared {
        /// Pipewire port id
        id: u32,
        /// parent node id
        node_id: u32,
        /// Input or Output
        direction: String,
        /// e.g. "FL", "FR", "MONO", etc.
        channel: String,
        /// `node.name` prop
        node_name: String,
    },

    /// Emitted when a Node proxy's info event fires.
    /// Full props post node init
    /// often includes `application.process.id` used to complete node
    NodeInfo {
        id: u32,
        app: Option<String>,
        pid: Option<u32>,
        nick: Option<String>,
    },

    /// used to react to `default-routes` and `default-configure-node` metadata
    MetadataAppeared {
        id: u32,
    },

    MetadataRemoved {
        id: u32,
    },

    /// only property listener produces this event
    MetadataProperty {
        subject: u32,
        /// prop key e.g. "default.rate
        key: Option<String>,
        value: Option<String>,
    },

    GlobalRemoved {
        id: u32,
    },
}

#[derive(Debug, Clone)]
pub enum Command {
    CreateLink {
        stereo_link: StereoLink,
    },

    DestroyLink {
        link: Link,
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
        node_id: u32,
    },

    CreateChannel {
        name: String,
    },

    CreateMix {
        name: String,
    },

    LoadInput {
        name: String,
        nick: String,
    },

    /// issues round-trip on pipewire
    CoreSync,

    DestroyNode {
        node_id: u32,
    },

    DeleteChannel {
        channel: Channel,
    },

    DeleteMix {
        mix: Mix,
    },
}
