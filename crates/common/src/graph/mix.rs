/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::{
    graph::node::Node,
    types::{ChannelPosition, MixName, PortDirection},
};

/// Mix node backed by a `PipeWire` `libpipewire-module-loopback`.
///
/// Contains a sink node (`pipelink.mix.{name}`) and a source node
/// (`pipelink.mix.{name}.source`)
#[derive(Clone, Debug, Default)]
pub struct Mix {
    pub name: MixName,
    pub sink: Node,
    pub source: Node,
}

impl Mix {
    /// Creates a new `Mix`.
    pub fn new(name: impl Into<MixName>) -> Self {
        let name = name.into();
        let sink = Node::default();
        let source = Node::default();

        Self { name, sink, source }
    }

    /// Returns whether this mix node is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        use ChannelPosition::{FrontLeft, FrontRight};
        use PortDirection::{Input, Output};

        if self.sink.id.is_none() || self.source.id.is_none() {
            return false;
        }

        self.sink.has_port(Input, FrontLeft)
            && self.sink.has_port(Input, FrontRight)
            && self.source.has_port(Output, FrontLeft)
            && self.source.has_port(Output, FrontRight)
    }
}
