/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use pipelink_common::types::{
    ChannelPosition, GlobalId, LinkId, MediaClass, NodeId, NodeName, NodeSerial, Pid,
    PortDirection, PortId,
};

/// Events emitted by the `PipeWire` background loop.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PipewireEvent {
    Ready,

    Disconnected {
        reason: String,
    },

    NodeAppeared {
        /// pipewire global id
        id: NodeId,
        /// `node.name` prop
        name: NodeName,
        /// `media.class` prop (e.g. "Audio/Sink", "Audio/Source")
        media_class: MediaClass,
        /// Gets resolved by the process resolver
        app: Option<String>,
        /// `application.process.id` required for the resolver
        pid: Option<Pid>,
        /// used for link generation
        serial: Option<NodeSerial>,
        /// `node.nick` prop
        nick: Option<String>,
    },

    NodeRemoved {
        id: NodeId,
    },

    LinkAppeared {
        /// Pipewire link id
        id: LinkId,
        /// Output node id
        out_node: NodeId,
        /// Input node id
        in_node: NodeId,
        /// Output port id
        out_port: PortId,
        /// Input port id
        in_port: PortId,
    },

    LinkRemoved {
        id: LinkId,
    },

    PortAppeared {
        /// Pipewire port id
        id: PortId,
        /// parent node id
        node_id: NodeId,
        /// Input or Output
        direction: PortDirection,
        /// e.g. "FL", "FR", "MONO", etc.
        channel: ChannelPosition,
    },

    /// Emitted when a Node proxy's info event fires.
    /// Full props post node init
    /// often includes `application.process.id` used to complete node
    NodeInfo {
        id: NodeId,
        /// `node.name` prop
        node_name: Option<NodeName>,
        app: Option<String>,
        pid: Option<Pid>,
        nick: Option<String>,
    },

    /// used to react to `default-routes` and `default-configure-node` metadata
    MetadataAppeared {
        id: GlobalId,
    },

    MetadataRemoved {
        id: GlobalId,
    },

    /// only property listener produces this event
    MetadataProperty {
        subject: GlobalId,
        /// prop key e.g. "default.rate
        key: Option<String>,
        value: Option<String>,
    },

    GlobalRemoved {
        id: GlobalId,
    },
}
