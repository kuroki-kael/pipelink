/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::{graph::node::Node, types::LinkId};

/// A single `PipeWire` link between a source and a sink node.
#[derive(Clone, Debug, Default)]
pub struct Link {
    pub link_id: LinkId,
    pub source: Node,
    pub sink: Node,
}

/// A stereo pair of [`Link`]s, one per channel.
#[derive(Clone, Debug, Default)]
pub struct StereoLink {
    pub fl: Link,
    pub fr: Link,
}
