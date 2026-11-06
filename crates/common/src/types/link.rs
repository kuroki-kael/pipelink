/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! `PipeWire` link global identifier.

use std::{borrow::Borrow, fmt, num::ParseIntError, str::FromStr};

use serde::{Deserialize, Serialize};

/// `PipeWire` link global identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct LinkId(pub u32);

impl From<u32> for LinkId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<LinkId> for u32 {
    fn from(id: LinkId) -> Self {
        id.0
    }
}

impl AsRef<u32> for LinkId {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl Borrow<u32> for LinkId {
    fn borrow(&self) -> &u32 {
        &self.0
    }
}

impl FromStr for LinkId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for LinkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
