/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::graph::node::Node;

#[derive(Clone, Debug)]
pub struct Link {
    pub link_id: u32,
    pub source: Node,
    pub sink: Node,
}

#[derive(Clone, Debug)]
pub struct StereoLink {
    pub fl: Link,
    pub fr: Link,
}
