/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Port models and shared naming for `PipeLink` `PipeWire` and `JACK` ports.

use crate::types::{ChannelName, ChannelPosition, MixName, PortDirection, PortId};

const CHANNEL_PREFIX: &str = "in";
const MIX_PREFIX: &str = "out";

/// Direction and channel position of a single registered `PipeWire` port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortInfo {
    /// Direction the port faces.
    pub direction: PortDirection,
    /// Channel position the port occupies within its node.
    pub position: ChannelPosition,
}

/// A front-left / front-right pair of `PipeWire` port IDs (e.g. for `JACK` ports).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StereoPortPair {
    pub fl: Option<PortId>,
    pub fr: Option<PortId>,
}

impl StereoPortPair {
    /// Returns `true` when both `FL` and `FR` port IDs are known.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.fl.is_some() && self.fr.is_some()
    }

    /// Stores `port_id` for `FrontLeft` or `FrontRight`, returning `true` if updated.
    pub fn set_port(&mut self, position: ChannelPosition, port_id: PortId) -> bool {
        match position {
            ChannelPosition::FrontLeft => {
                self.fl = Some(port_id);
                true
            },
            ChannelPosition::FrontRight => {
                self.fr = Some(port_id);
                true
            },
            _ => false,
        }
    }
}

/// Logical identity of a port exposed by the `PipeLink` `JACK` client.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MixerPort {
    /// Stereo channel input port (`in_{channel}_FL` / `in_{channel}_FR`).
    ChannelInput {
        /// Owning channel name.
        channel: ChannelName,
        /// Channel side (`FrontLeft` or `FrontRight`).
        position: ChannelPosition,
    },
    /// Stereo mix output port (`out_{mix}_FL` / `out_{mix}_FR`).
    MixOutput {
        /// Owning mix name.
        mix: MixName,
        /// Channel side (`FrontLeft` or `FrontRight`).
        position: ChannelPosition,
    },
}

impl MixerPort {
    /// Returns the `JACK` / `PipeWire` `port.name`, e.g. `in_Music_FL` or `out_Stream_FR`.
    #[must_use]
    pub fn name(&self) -> String {
        let (prefix, owner, position) = match self {
            Self::ChannelInput { channel, position } => {
                (CHANNEL_PREFIX, channel.as_ref(), *position)
            },
            Self::MixOutput { mix, position } => (MIX_PREFIX, mix.as_ref(), *position),
        };
        format!("{prefix}_{owner}_{position}")
    }

    /// Parses a `PipeWire` `port.name` belonging to the `PipeLink` `JACK` client.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let (prefix, rest) = name.split_once('_')?;
        let (owner, position) = rest.rsplit_once('_')?;
        if owner.is_empty() {
            return None;
        }
        let position = match position {
            "FL" => ChannelPosition::FrontLeft,
            "FR" => ChannelPosition::FrontRight,
            _ => return None,
        };

        match prefix {
            CHANNEL_PREFIX => Some(Self::ChannelInput {
                channel: ChannelName::from(owner),
                position,
            }),
            MIX_PREFIX => Some(Self::MixOutput {
                mix: MixName::from(owner),
                position,
            }),
            _ => None,
        }
    }

    /// Returns the port direction (`Input` for channels, `Output` for mixes).
    #[must_use]
    pub const fn direction(&self) -> PortDirection {
        match self {
            Self::ChannelInput { .. } => PortDirection::Input,
            Self::MixOutput { .. } => PortDirection::Output,
        }
    }

    /// Returns the stereo position (`FrontLeft` or `FrontRight`).
    #[must_use]
    pub const fn position(&self) -> ChannelPosition {
        match self {
            Self::ChannelInput { position, .. } | Self::MixOutput { position, .. } => *position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixer_port_round_trip_and_underscores() {
        let ch = MixerPort::ChannelInput {
            channel: ChannelName::from("game_audio"),
            position: ChannelPosition::FrontLeft,
        };
        assert_eq!(ch.name(), "in_game_audio_FL");
        assert_eq!(MixerPort::from_name(&ch.name()), Some(ch));

        let mx = MixerPort::MixOutput {
            mix: MixName::from("stream_mix"),
            position: ChannelPosition::FrontRight,
        };
        assert_eq!(mx.name(), "out_stream_mix_FR");
        assert_eq!(MixerPort::from_name(&mx.name()), Some(mx));
    }

    #[test]
    fn test_mixer_port_rejects_invalid_names() {
        assert_eq!(MixerPort::from_name("in__FL"), None);
        assert_eq!(MixerPort::from_name("out__FR"), None);
        assert_eq!(MixerPort::from_name("in_Game_MONO"), None);
        assert_eq!(MixerPort::from_name("playback_FL"), None);
        assert_eq!(MixerPort::from_name("in_Game"), None);
    }

    #[test]
    fn test_mixer_port_direction_and_position() {
        let ch = MixerPort::ChannelInput {
            channel: ChannelName::from("Game"),
            position: ChannelPosition::FrontLeft,
        };
        let mx = MixerPort::MixOutput {
            mix: MixName::from("Main"),
            position: ChannelPosition::FrontRight,
        };
        assert_eq!(ch.direction(), PortDirection::Input);
        assert_eq!(ch.position(), ChannelPosition::FrontLeft);
        assert_eq!(mx.direction(), PortDirection::Output);
        assert_eq!(mx.position(), ChannelPosition::FrontRight);
    }

    #[test]
    fn test_stereo_port_pair_set_and_complete() {
        let mut pair = StereoPortPair::default();
        assert!(!pair.is_complete());
        assert!(!pair.set_port(ChannelPosition::Mono, PortId(1)));
        assert!(pair.set_port(ChannelPosition::FrontLeft, PortId(2)));
        assert!(!pair.is_complete());
        assert!(pair.set_port(ChannelPosition::FrontRight, PortId(3)));
        assert!(pair.is_complete());
    }
}
