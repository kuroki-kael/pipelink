/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use jack::{AudioIn, AudioOut, Port};

pub type ChannelPorts = ChannelInputPorts;
pub type MixPorts = MixOutputPorts;

/// Stereo input ports for a channel (e.g. "Game", "Music", "Voice").
#[derive(Debug)]
pub struct ChannelInputPorts {
    pub in_l: Port<AudioIn>,
    pub in_r: Port<AudioIn>,
}

impl ChannelInputPorts {
    pub fn new(in_l: Port<AudioIn>, in_r: Port<AudioIn>) -> Self {
        Self { in_l, in_r }
    }

    /// Registers stereo input ports (`in_{channel_name}_FL` and `in_{channel_name}_FR`)
    /// on a JACK client.
    ///
    /// # Errors
    ///
    /// Returns a [`jack::Error`] if registration failed.
    pub fn register(client: &jack::Client, channel_name: &str) -> Result<Self, jack::Error> {
        let in_l = client.register_port(&format!("in_{channel_name}_FL"), AudioIn::default())?;
        let in_r = client.register_port(&format!("in_{channel_name}_FR"), AudioIn::default())?;
        Ok(Self { in_l, in_r })
    }

    /// Returns references to the Left and Right audio sample slices for the current
    /// process cycle.
    #[inline]
    pub fn buffers<'a>(&'a self, ps: &'a jack::ProcessScope) -> (&'a [f32], &'a [f32]) {
        (self.in_l.as_slice(ps), self.in_r.as_slice(ps))
    }
}

/// Stereo output ports for a mix (e.g. "Personal", "Stream").
#[derive(Debug)]
pub struct MixOutputPorts {
    pub out_l: Port<AudioOut>,
    pub out_r: Port<AudioOut>,
}

impl MixOutputPorts {
    pub fn new(out_l: Port<AudioOut>, out_r: Port<AudioOut>) -> Self {
        Self { out_l, out_r }
    }

    /// Registers stereo output ports (`out_{mix_name}_FL` and `out_{mix_name}_FR`) on a
    /// JACK client.
    ///
    /// # Errors
    ///
    /// Returns a [`jack::Error`] if registration failed.
    pub fn register(client: &jack::Client, mix_name: &str) -> Result<Self, jack::Error> {
        let out_l = client.register_port(&format!("out_{mix_name}_FL"), AudioOut::default())?;
        let out_r = client.register_port(&format!("out_{mix_name}_FR"), AudioOut::default())?;
        Ok(Self { out_l, out_r })
    }

    /// Returns mutable references to the Left and Right audio sample slices for the
    /// current process cycle.
    #[inline]
    pub fn buffers_mut<'a>(
        &'a mut self,
        ps: &'a jack::ProcessScope,
    ) -> (&'a mut [f32], &'a mut [f32]) {
        (self.out_l.as_mut_slice(ps), self.out_r.as_mut_slice(ps))
    }

    /// Clears both output buffers by filling them with silence (`0.0`).
    #[inline]
    pub fn clear(&mut self, ps: &jack::ProcessScope) {
        self.out_l.as_mut_slice(ps).fill(0.0);
        self.out_r.as_mut_slice(ps).fill(0.0);
    }
}
