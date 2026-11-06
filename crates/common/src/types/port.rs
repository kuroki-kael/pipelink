/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! `PipeWire` port identifiers, directions and channel positions.

use std::{borrow::Borrow, fmt, num::ParseIntError, str::FromStr};

use serde::{Deserialize, Serialize};

/// `PipeWire` port global identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct PortId(pub u32);

impl From<u32> for PortId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<PortId> for u32 {
    fn from(id: PortId) -> Self {
        id.0
    }
}

impl AsRef<u32> for PortId {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl Borrow<u32> for PortId {
    fn borrow(&self) -> &u32 {
        &self.0
    }
}

impl FromStr for PortId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for PortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Direction of an audio port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PortDirection {
    /// Direction is unknown or not applicable.
    #[default]
    None,
    Input,
    Output,
}

impl PortDirection {
    /// Parses a `port.direction` value as reported by `PipeWire`.
    #[must_use]
    pub fn from_pw_str(s: &str) -> Self {
        match s {
            "in" | "Input" => Self::Input,
            "out" | "Output" => Self::Output,
            _ => Self::None,
        }
    }
}

impl fmt::Display for PortDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Input => write!(f, "Input"),
            Self::Output => write!(f, "Output"),
        }
    }
}

/// Standardized audio channel positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ChannelPosition {
    /// Position not covered by a dedicated variant.
    #[default]
    Other,
    FrontLeft,
    FrontRight,
    FrontCenter,
    RearLeft,
    RearRight,
    Lfe,
    Mono,
    /// Auxiliary channel position, e.g. `AUX0` .. `AUX7`.
    Aux(u8),
}

impl ChannelPosition {
    /// Parses an `audio.channel` value as reported by `PipeWire`.
    #[must_use]
    pub fn from_pw_str(s: &str) -> Self {
        match s {
            "FL" => Self::FrontLeft,
            "FR" => Self::FrontRight,
            "FC" => Self::FrontCenter,
            "RL" => Self::RearLeft,
            "RR" => Self::RearRight,
            "LFE" => Self::Lfe,
            "MONO" => Self::Mono,
            _ => Self::from_aux_str(s),
        }
    }

    /// Parses an auxiliary channel position such as `AUX0` or `AUX7`.
    #[must_use]
    fn from_aux_str(s: &str) -> Self {
        s.strip_prefix("AUX")
            .and_then(|index| index.parse::<u8>().ok())
            .map_or(Self::Other, Self::Aux)
    }
}

impl fmt::Display for ChannelPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrontLeft => write!(f, "FL"),
            Self::FrontRight => write!(f, "FR"),
            Self::FrontCenter => write!(f, "FC"),
            Self::RearLeft => write!(f, "RL"),
            Self::RearRight => write!(f, "RR"),
            Self::Lfe => write!(f, "LFE"),
            Self::Mono => write!(f, "MONO"),
            Self::Aux(index) => write!(f, "AUX{index}"),
            Self::Other => write!(f, "Other"),
        }
    }
}
