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
    cmd_rx: Consumer<Command>,
    retired_tx: Producer<RetiredResource>,
}

impl JackProcess {
    /// Creates a new process handler with pre-reserved channel and mix port storage.
    pub(crate) fn new(cmd_rx: Consumer<Command>, retired_tx: Producer<RetiredResource>) -> Self {
        Self {
            channels: Vec::with_capacity(INITIAL_PORT_CAPACITY),
            mixes: Vec::with_capacity(INITIAL_PORT_CAPACITY),
            gains: Vec::new(),
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

    /// Applies a single control command using the pre-built gains slice from the engine.
    fn apply_command(&mut self, cmd: Command) {
        match cmd {
            Command::CreateChannel { ports, gains } => {
                self.channels.push(ports);
                self.replace_gains(gains);
            },
            Command::RemoveChannel { idx, gains } => {
                if idx < self.channels.len() {
                    let old_ports = self.channels.remove(idx);
                    let _ = self.retired_tx.push(RetiredResource::Channel(old_ports));
                }
                self.replace_gains(gains);
            },
            Command::MoveChannel {
                from_idx,
                to_idx,
                gains,
            } => {
                if from_idx < self.channels.len() && to_idx < self.channels.len() {
                    let ports = self.channels.remove(from_idx);
                    self.channels.insert(to_idx, ports);
                }
                self.replace_gains(gains);
            },
            Command::CreateMix { ports, gains } => {
                self.mixes.push(ports);
                self.replace_gains(gains);
            },
            Command::RemoveMix { idx, gains } => {
                if idx < self.mixes.len() {
                    let old_ports = self.mixes.remove(idx);
                    let _ = self.retired_tx.push(RetiredResource::Mix(old_ports));
                }
                self.replace_gains(gains);
            },
            Command::MoveMix {
                from_idx,
                to_idx,
                gains,
            } => {
                if from_idx < self.mixes.len() && to_idx < self.mixes.len() {
                    let ports = self.mixes.remove(from_idx);
                    self.mixes.insert(to_idx, ports);
                }
                self.replace_gains(gains);
            },
            Command::SetGains(gains) => {
                self.replace_gains(gains);
            },
        }
    }

    #[inline]
    fn replace_gains(&mut self, new_gains: Vec<f32>) {
        let old_gains = std::mem::replace(&mut self.gains, new_gains);
        if old_gains.capacity() > 0 {
            let _ = self.retired_tx.push(RetiredResource::Gains(old_gains));
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
    fn test_set_gains_replaces_slice_and_retires_previous_buffer() {
        let (_cmd_tx, cmd_rx) = RingBuffer::new(8);
        let (retired_tx, mut retired_rx) = RingBuffer::new(8);
        let mut process = JackProcess::new(cmd_rx, retired_tx);

        process.apply_command(Command::SetGains(vec![1.0, 0.5]));
        assert_eq!(process.gains, vec![1.0, 0.5]);
        assert!(retired_rx.pop().is_err());

        process.apply_command(Command::SetGains(vec![0.25, 0.75]));
        assert_eq!(process.gains, vec![0.25, 0.75]);
        assert!(matches!(
            retired_rx.pop(),
            Ok(RetiredResource::Gains(old)) if old == vec![1.0, 0.5]
        ));
    }

    #[test]
    fn test_drain_commands_handles_out_of_bounds_removals_safely() {
        let (mut cmd_tx, cmd_rx) = RingBuffer::new(8);
        let (retired_tx, _retired_rx) = RingBuffer::new(8);
        let mut process = JackProcess::new(cmd_rx, retired_tx);

        assert!(
            cmd_tx
                .push(Command::RemoveChannel {
                    idx: 0,
                    gains: Vec::new()
                })
                .is_ok()
        );
        assert!(
            cmd_tx
                .push(Command::RemoveMix {
                    idx: 0,
                    gains: Vec::new()
                })
                .is_ok()
        );
        process.drain_commands();

        assert!(process.channels.is_empty());
        assert!(process.mixes.is_empty());
        assert_eq!(process.gains, [] as [f32; 0]);
    }
}
