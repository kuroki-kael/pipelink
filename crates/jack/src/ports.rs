/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use jack::{AudioIn, AudioOut, Port};
use pipelink_common::{
    graph::port::MixerPort,
    types::{ChannelName, ChannelPosition, MixName},
};

/// Alias for [`ChannelInputPorts`].
pub type ChannelPorts = ChannelInputPorts;
/// Alias for [`MixOutputPorts`].
pub type MixPorts = MixOutputPorts;

/// Stereo input ports for a channel (e.g. "Game", "Music", "Voice").
#[derive(Debug)]
pub struct ChannelInputPorts {
    pub in_l: Port<AudioIn>,
    pub in_r: Port<AudioIn>,
}

impl ChannelInputPorts {
    /// Creates a stereo input port pair from two JACK ports.
    pub fn new(in_l: Port<AudioIn>, in_r: Port<AudioIn>) -> Self {
        Self { in_l, in_r }
    }

    /// Registers stereo input ports (`in_{channel_name}_FL` and `in_{channel_name}_FR`)
    /// on a JACK client.
    ///
    /// # Errors
    ///
    /// Returns a [`jack::Error`] if registration failed.
    pub fn register(client: &jack::Client, channel_name: &ChannelName) -> Result<Self, jack::Error> {
        let left_name = MixerPort::ChannelInput {
            channel: channel_name.clone(),
            position: ChannelPosition::FrontLeft,
        }
        .name();
        let right_name = MixerPort::ChannelInput {
            channel: channel_name.clone(),
            position: ChannelPosition::FrontRight,
        }
        .name();

        let in_l = client.register_port(&left_name, AudioIn::default())?;
        let in_r = match client.register_port(&right_name, AudioIn::default()) {
            Ok(port) => port,
            Err(err) => {
                let _ = client.unregister_port(in_l);
                return Err(err);
            },
        };
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
    /// Creates a stereo output port pair from two JACK ports.
    pub fn new(out_l: Port<AudioOut>, out_r: Port<AudioOut>) -> Self {
        Self { out_l, out_r }
    }

    /// Registers stereo output ports (`out_{mix_name}_FL` and `out_{mix_name}_FR`) on a
    /// JACK client.
    ///
    /// # Errors
    ///
    /// Returns a [`jack::Error`] if registration failed.
    pub fn register(client: &jack::Client, mix_name: &MixName) -> Result<Self, jack::Error> {
        let left_name = MixerPort::MixOutput {
            mix: mix_name.clone(),
            position: ChannelPosition::FrontLeft,
        }
        .name();
        let right_name = MixerPort::MixOutput {
            mix: mix_name.clone(),
            position: ChannelPosition::FrontRight,
        }
        .name();

        let out_l = client.register_port(&left_name, AudioOut::default())?;
        let out_r = match client.register_port(&right_name, AudioOut::default()) {
            Ok(port) => port,
            Err(err) => {
                let _ = client.unregister_port(out_l);
                return Err(err);
            },
        };
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
