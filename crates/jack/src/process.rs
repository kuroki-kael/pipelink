/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Real-time JACK `ProcessHandler` performing channel-to-mix summation.

use jack::{Client, Control, ProcessHandler, ProcessScope};
use rtrb::{Consumer, Producer};

use crate::{
    Command, RetiredResource,
    ports::{ChannelPorts, MixPorts},
};

const INITIAL_PORT_CAPACITY: usize = 32;

/// Real-time audio mixer running inside the JACK process callback.
pub(crate) struct JackProcess {
    channels: Vec<ChannelPorts>,
    mixes: Vec<MixPorts>,
    gains: Vec<f32>,
    scratch_gains: Vec<f32>,
    cmd_rx: Consumer<Command>,
    retired_tx: Producer<RetiredResource>,
}

impl JackProcess {
    /// Creates a new process handler with pre-reserved channel, mix, and matrix storage.
    pub(crate) fn new(
        cmd_rx: Consumer<Command>,
        retired_tx: Producer<RetiredResource>,
    ) -> Self {
        Self {
            channels: Vec::with_capacity(INITIAL_PORT_CAPACITY),
            mixes: Vec::with_capacity(INITIAL_PORT_CAPACITY),
            gains: Vec::with_capacity(INITIAL_PORT_CAPACITY * INITIAL_PORT_CAPACITY),
            scratch_gains: Vec::with_capacity(INITIAL_PORT_CAPACITY * INITIAL_PORT_CAPACITY),
            cmd_rx,
            retired_tx,
        }
    }

    /// Drains all pending control commands without blocking.
    fn drain_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.pop() {
            self.apply_command(cmd);
        }
    }

    /// Applies a single control command while keeping `gains` aligned with `channels × mixes`.
    fn apply_command(&mut self, cmd: Command) {
        match cmd {
            Command::CreateChannel(ports) => {
                self.channels.push(ports);
                self.gains.resize(self.channels.len() * self.mixes.len(), 0.0);
            },
            Command::RemoveChannel(idx) => {
                if idx < self.channels.len() {
                    let num_mixes = self.mixes.len();
                    let row_start = idx * num_mixes;
                    self.gains.drain(row_start..row_start + num_mixes);

                    let old_ports = self.channels.remove(idx);
                    let _ = self.retired_tx.push(RetiredResource::Channel(old_ports));
                }
            },
            Command::CreateMix(ports) => {
                let old_cols = self.mixes.len();
                let num_channels = self.channels.len();

                self.scratch_gains.clear();
                for row in 0..num_channels {
                    let start = row * old_cols;
                    self.scratch_gains
                        .extend_from_slice(&self.gains[start..start + old_cols]);
                    self.scratch_gains.push(0.0);
                }

                std::mem::swap(&mut self.gains, &mut self.scratch_gains);
                self.mixes.push(ports);
            },
            Command::RemoveMix(idx) => {
                if idx < self.mixes.len() {
                    let old_cols = self.mixes.len();
                    let num_channels = self.channels.len();

                    for row in (0..num_channels).rev() {
                        self.gains.remove(row * old_cols + idx);
                    }

                    let old_ports = self.mixes.remove(idx);
                    let _ = self.retired_tx.push(RetiredResource::Mix(old_ports));
                }
            },
            Command::SetRoute {
                channel_idx,
                mix_idx,
                gain,
            } => {
                self.set_route_gain(channel_idx, mix_idx, gain);
            },
        }
    }

    #[inline]
    fn set_route_gain(&mut self, channel_idx: usize, mix_idx: usize, gain: f32) {
        let num_mixes = self.mixes.len();
        if channel_idx < self.channels.len() && mix_idx < num_mixes {
            self.gains[channel_idx * num_mixes + mix_idx] = gain;
        }
    }

    /// Removes and returns all remaining ports during client shutdown.
    pub(crate) fn drain_all_ports(&mut self) -> (Vec<ChannelPorts>, Vec<MixPorts>) {
        self.gains.clear();
        (
            std::mem::take(&mut self.channels),
            std::mem::take(&mut self.mixes),
        )
    }
}

impl ProcessHandler for JackProcess {
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> Control {
        self.drain_commands();

        for mix in &mut self.mixes {
            mix.clear(ps);
        }

        let num_mixes = self.mixes.len();
        if self.gains.len() != self.channels.len() * num_mixes {
            return Control::Continue;
        }

        for (mix_idx, mix) in self.mixes.iter_mut().enumerate() {
            let (out_l, out_r) = mix.buffers_mut(ps);

            for (ch_idx, ch) in self.channels.iter().enumerate() {
                let gain = self.gains[ch_idx * num_mixes + mix_idx];
                if gain == 0.0 {
                    continue;
                }

                let (in_l, in_r) = ch.buffers(ps);
                mix_stereo_into(in_l, in_r, out_l, out_r, gain);
            }
        }

        Control::Continue
    }
}

/// Accumulates a stereo input pair scaled by `gain` into a stereo output pair.
#[inline]
fn mix_stereo_into(in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32], gain: f32) {
    for (out_sample, &in_sample) in out_l.iter_mut().zip(in_l) {
        *out_sample += in_sample * gain;
    }
    for (out_sample, &in_sample) in out_r.iter_mut().zip(in_r) {
        *out_sample += in_sample * gain;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use rtrb::RingBuffer;

    use super::*;

    #[test]
    fn test_mix_stereo_into_accumulates_scaled_samples() {
        let in_l = [1.0, -0.5, 0.25, 0.0];
        let in_r = [0.5, 1.0, -0.25, 0.5];
        let mut out_l = [0.1, 0.1, 0.1, 0.1];
        let mut out_r = [0.0; 4];

        mix_stereo_into(&in_l, &in_r, &mut out_l, &mut out_r, 0.5);

        assert_eq!(out_l, [0.6, -0.15, 0.225, 0.1]);
        assert_eq!(out_r, [0.25, 0.5, -0.125, 0.25]);
    }

    #[test]
    fn test_set_route_gain_ignores_out_of_bounds_without_panicking() {
        let (_cmd_tx, cmd_rx) = RingBuffer::new(8);
        let (retired_tx, _retired_rx) = RingBuffer::new(8);
        let mut process = JackProcess::new(cmd_rx, retired_tx);
        process.gains = vec![1.0, 0.0];

        process.apply_command(Command::SetRoute {
            channel_idx: 0,
            mix_idx: 0,
            gain: 0.5,
        });
        assert_eq!(process.gains, vec![1.0, 0.0]);
    }

    #[test]
    fn test_drain_commands_processes_route_updates_when_channels_empty() {
        let (mut cmd_tx, cmd_rx) = RingBuffer::new(8);
        let (retired_tx, _retired_rx) = RingBuffer::new(8);
        let mut process = JackProcess::new(cmd_rx, retired_tx);

        assert!(cmd_tx.push(Command::RemoveChannel(0)).is_ok());
        assert!(cmd_tx.push(Command::RemoveMix(0)).is_ok());
        process.drain_commands();

        assert!(process.channels.is_empty());
        assert!(process.mixes.is_empty());
        assert_eq!(process.gains, [] as [f32; 0]);
    }
}
