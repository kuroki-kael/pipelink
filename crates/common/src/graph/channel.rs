/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::{
    graph::node::Node,
    types::{ChannelName, ChannelPosition, PortDirection},
};

/// Channel node backed by a `Pipewire` `support.null-audio-sink` adapter.
#[derive(Debug, Clone, Default)]
pub struct Channel {
    pub name: ChannelName,
    pub node: Node,
}

impl Channel {
    /// Creates a mew `Channel`.
    pub fn new(name: impl Into<ChannelName>) -> Self {
        let name = name.into();
        let node = Node::default();
        Self { name, node }
    }

    /// Returns whether this channel node is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        use ChannelPosition::{FrontLeft, FrontRight};
        use PortDirection::{Input, Output};

        if !self.node.is_ready() {
            return false;
        }

        self.node.has_port(Input, FrontLeft)
            && self.node.has_port(Input, FrontRight)
            && self.node.has_port(Output, FrontLeft)
            && self.node.has_port(Output, FrontRight)
    }
}
