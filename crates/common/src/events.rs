/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Domain events for audio routing control and state notifications.

use serde::{Deserialize, Serialize};

use crate::types::{ChannelName, MixName};

/// Events sent from controllers (IPC, CLI, GUI, MIDI, Web) to request state changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlEvent {
    /// Sets channel gain for a specific mix.
    SetChannelGain {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
        /// Gain value in decibels.
        db_value: f32,
    },
    /// Sets mute status of a channel for a specific mix.
    SetChannelMute {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
        /// Whether the channel should be muted.
        muted: bool,
    },
    /// Requests creation of a new audio channel.
    CreateChannel {
        /// Channel name to create.
        name: ChannelName,
    },
    /// Requests removal of an audio channel.
    RemoveChannel {
        /// Channel name to remove.
        name: ChannelName,
    },
    /// Requests creation of a new audio mix.
    CreateMix {
        /// Mix name to create.
        name: MixName,
    },
    /// Requests removal of an audio mix.
    RemoveMix {
        /// Mix name to remove.
        name: MixName,
    },
    /// Selects the global active mix layer.
    SelectGlobalActiveLayer {
        /// Mix name to activate globally.
        mix: MixName,
    },
    /// Selects the active layer for a specific channel.
    SelectChannelActiveLayer {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
    },
    /// Adds a channel to a mix.
    AddChannelMix {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
    },
    /// Removes a channel from a mix.
    RemoveChannelMix {
        /// Target channel name.
        channel: ChannelName,
        /// Target mix name.
        mix: MixName,
    },
}

/// Events broadcast by the audio engine to notify listeners of state changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateChangeEvent {
    /// Gain changed for a channel within a mix.
    GainChanged {
        /// Affected channel name.
        channel: ChannelName,
        /// Affected mix name.
        mix: MixName,
        /// Linear gain multiplier.
        gain_linear: f32,
        /// Gain value in decibels.
        db_value: f32,
    },
    /// Mute state changed for a channel within a mix.
    MuteChanged {
        /// Affected channel name.
        channel: ChannelName,
        /// Affected mix name.
        mix: MixName,
        /// Whether the channel is muted.
        muted: bool,
    },
    /// Global active layer switched to a new mix.
    GlobalLayerSwitched {
        /// New active mix name.
        mix: MixName,
    },
    /// Active layer switched for a channel.
    ChannelLayerSwitched {
        /// Affected channel name.
        channel: ChannelName,
        /// Affected mix name.
        mix: MixName,
    },
    /// New channel created.
    ChannelAdded {
        /// Channel name.
        channel: ChannelName,
    },
    /// Channel deleted.
    ChannelRemoved {
        /// Channel name.
        channel: ChannelName,
    },
    /// New mix created.
    MixAdded {
        /// Mix name.
        mix: MixName,
    },
    /// Mix deleted.
    MixRemoved {
        /// Mix name.
        mix: MixName,
    },
    /// Channel associated with a mix.
    ChannelMixAdded {
        /// Channel name.
        channel: ChannelName,
        /// Mix name.
        mix: MixName,
    },
    /// Channel dissociated from a mix.
    ChannelMixRemoved {
        /// Channel name.
        channel: ChannelName,
        /// Mix name.
        mix: MixName,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{ControlEvent, StateChangeEvent};
    use crate::types::{ChannelName, MixName};

    #[test]
    fn test_control_event_json_format() {
        let event = ControlEvent::SetChannelGain {
            channel: ChannelName::from("mic"),
            mix: MixName::from("headphones"),
            db_value: -6.0,
        };

        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"type":"set_channel_gain","channel":"mic","mix":"headphones","db_value":-6.0}"#
        );
    }

    #[test]
    fn test_control_event_create_channel_json_format() {
        let event = ControlEvent::CreateChannel {
            name: ChannelName::from("mic"),
        };

        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"type":"create_channel","name":"mic"}"#
        );
    }

    #[test]
    fn test_state_change_event_json_format() {
        let event = StateChangeEvent::ChannelAdded {
            channel: ChannelName::from("mic"),
        };

        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"type":"channel_added","channel":"mic"}"#
        );
    }

    #[test]
    fn test_events_round_trip() {
        let event = ControlEvent::AddChannelMix {
            channel: ChannelName::from("mic"),
            mix: MixName::from("headphones"),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: ControlEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, event);
    }
}
