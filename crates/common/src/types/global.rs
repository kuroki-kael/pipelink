/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Identifier of any `PipeWire` global object.

use std::{borrow::Borrow, fmt, num::ParseIntError, str::FromStr};

use serde::{Deserialize, Serialize};

use super::node::NodeId;

/// `PipeWire` global object identifier.
///
/// `PipeWire` draws node, port, link and metadata ids from a single id space,
/// so a [`GlobalId`] identifies exactly one global object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct GlobalId(pub u32);

impl From<u32> for GlobalId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<GlobalId> for u32 {
    fn from(id: GlobalId) -> Self {
        id.0
    }
}

impl AsRef<u32> for GlobalId {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl Borrow<u32> for GlobalId {
    fn borrow(&self) -> &u32 {
        &self.0
    }
}

impl FromStr for GlobalId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for GlobalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<GlobalId> for NodeId {
    /// Views a global id as a node id.
    ///
    /// `PipeWire` shares one id space across all global kinds, so a global id
    /// denotes a node exactly when the node carries that id.
    fn from(id: GlobalId) -> Self {
        Self(id.0)
    }
}
