/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! `PipeWire` node identifiers, names and serial numbers.

use std::{borrow::Borrow, fmt, num::ParseIntError, ops::Deref, str::FromStr};

use serde::{Deserialize, Serialize};

/// `PipeWire` node global identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct NodeId(pub u32);

impl From<u32> for NodeId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<NodeId> for u32 {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

impl AsRef<u32> for NodeId {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl Borrow<u32> for NodeId {
    fn borrow(&self) -> &u32 {
        &self.0
    }
}

impl FromStr for NodeId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Canonical `PipeWire` node name or slug.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeName(pub String);

impl From<String> for NodeName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for NodeName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<NodeName> for String {
    fn from(name: NodeName) -> Self {
        name.0
    }
}

impl fmt::Display for NodeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for NodeName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for NodeName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for NodeName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for NodeName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for NodeName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<NodeName> for &str {
    fn eq(&self, other: &NodeName) -> bool {
        *self == other.0
    }
}

impl PartialEq<NodeName> for str {
    fn eq(&self, other: &NodeName) -> bool {
        *self == other.0
    }
}

/// `PipeWire` 64-bit object serial number (used for `target.object`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct NodeSerial(pub u64);

impl From<u64> for NodeSerial {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<NodeSerial> for u64 {
    fn from(serial: NodeSerial) -> Self {
        serial.0
    }
}

impl AsRef<u64> for NodeSerial {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}

impl Borrow<u64> for NodeSerial {
    fn borrow(&self) -> &u64 {
        &self.0
    }
}

impl FromStr for NodeSerial {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for NodeSerial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
