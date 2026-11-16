/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::types::{ChannelName, MixName};

/// Routing entry pairing a single channel with a single mix in the N:M matrix.
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelMix {
    /// Source channel of this route.
    pub channel: ChannelName,
    /// Destination mix of this route.
    pub mix: MixName,
    /// Whether audio is currently routed from `channel` into `mix`.
    pub active: bool,
    /// Linear gain multiplier applied to this route (`1.0` = unity gain).
    pub gain: f32,
    /// Whether this route is muted.
    pub muted: bool,
}

impl Default for ChannelMix {
    fn default() -> Self {
        Self {
            channel: ChannelName::default(),
            mix: MixName::default(),
            active: false,
            gain: 1.0,
            muted: false,
        }
    }
}

impl ChannelMix {
    /// Creates an inactive, unmuted routing entry at unity gain (`1.0`).
    pub fn new(channel: ChannelName, mix: MixName) -> Self {
        Self {
            channel,
            mix,
            ..Default::default()
        }
    }

    /// Returns the effective linear gain (`0.0` when inactive or muted).
    #[inline]
    #[must_use]
    pub fn effective_gain(&self) -> f32 {
        if self.active && !self.muted {
            self.gain
        } else {
            0.0
        }
    }
}
