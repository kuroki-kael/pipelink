/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Names of the routing layers (channels and mixes).

use std::{borrow::Borrow, fmt, ops::Deref};

use serde::{Deserialize, Serialize};

/// Name of a routing channel.
///
/// A channel is the virtual sink an application is routed to. This is the
/// user-facing name, not the `PipeWire` node name of the backing null sink.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelName(pub String);

impl From<String> for ChannelName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ChannelName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<ChannelName> for String {
    fn from(name: ChannelName) -> Self {
        name.0
    }
}

impl fmt::Display for ChannelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for ChannelName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for ChannelName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for ChannelName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for ChannelName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for ChannelName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<ChannelName> for &str {
    fn eq(&self, other: &ChannelName) -> bool {
        *self == other.0
    }
}

impl PartialEq<ChannelName> for str {
    fn eq(&self, other: &ChannelName) -> bool {
        *self == other.0
    }
}

/// Name of a routing mix.
///
/// A mix consumes one or more channels. This is the user-facing name, not the
/// `PipeWire` node name of the backing loopback module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MixName(pub String);

impl From<String> for MixName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MixName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<MixName> for String {
    fn from(name: MixName) -> Self {
        name.0
    }
}

impl fmt::Display for MixName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for MixName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for MixName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for MixName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for MixName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for MixName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<MixName> for &str {
    fn eq(&self, other: &MixName) -> bool {
        *self == other.0
    }
}

impl PartialEq<MixName> for str {
    fn eq(&self, other: &MixName) -> bool {
        *self == other.0
    }
}
