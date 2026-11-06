/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Audio routing topology models and configuration graph.

use pipelink_common::types::{AppId, ChannelName, MixName, NodeName};
use serde::{Deserialize, Serialize};

/// A virtual Sink an app or microphone gets routed to.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub struct AudioChannel {
    /// Channel name.
    pub name: ChannelName,
    /// App identifiers routed to this channel.
    #[serde(default)]
    pub apps: Vec<AppId>,
    /// Optional physical input source.
    #[serde(default)]
    pub source: Option<NodeName>,
}

/// A consumer of one or more channels.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub struct AudioMix {
    /// Mix name.
    pub name: MixName,
    /// Channels routed to mix.
    #[serde(default)]
    pub channels: Vec<ChannelName>,
}

/// In-memory and serialized representation of the audio routing graph.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConfigGraph {
    /// Physical Sink name the active mix routes to.
    #[serde(default)]
    pub output: Option<NodeName>,
    /// Name of mix that goes to output.
    #[serde(default)]
    pub active_mix: Option<MixName>,
    /// All defined channels.
    #[serde(default)]
    pub channels: Vec<AudioChannel>,
    /// All defined mixes.
    #[serde(default)]
    pub mixes: Vec<AudioMix>,
}

impl ConfigGraph {
    /// Creates a new empty [`ConfigGraph`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the active mix.
    pub fn set_active_mix(&mut self, active_mix: impl Into<MixName>) {
        self.active_mix = Some(active_mix.into());
    }

    /// Returns the active mix if set.
    #[must_use]
    pub fn get_active_mix(&self) -> Option<&str> {
        self.active_mix.as_deref()
    }

    /// Sets the physical output.
    pub fn set_output(&mut self, output: impl Into<NodeName>) {
        self.output = Some(output.into());
    }

    /// Returns the physical output if set.
    #[must_use]
    pub fn get_output(&self) -> Option<&str> {
        self.output.as_deref()
    }

    /// Adds a channel if it does not already exist.
    pub fn add_channel(&mut self, name: impl Into<ChannelName>) {
        let name = name.into();
        if !self.channels.iter().any(|c| c.name == name) {
            self.channels.push(AudioChannel {
                name,
                ..Default::default()
            });
        }
    }

    /// Returns a reference to a channel by name.
    #[must_use]
    pub fn get_channel(&self, name: &str) -> Option<&AudioChannel> {
        self.channels.iter().find(|c| c.name == name)
    }

    /// Returns a mutable reference to a channel by name.
    pub fn get_channel_mut(&mut self, name: &str) -> Option<&mut AudioChannel> {
        self.channels.iter_mut().find(|c| c.name == name)
    }

    /// Returns all channel names.
    #[must_use]
    pub fn get_all_channels(&self) -> Vec<ChannelName> {
        self.channels.iter().map(|c| c.name.clone()).collect()
    }

    /// Adds a mix if it does not already exist.
    pub fn add_mix(&mut self, name: impl Into<MixName>) {
        let name = name.into();
        if !self.mixes.iter().any(|c| c.name == name) {
            self.mixes.push(AudioMix {
                name,
                ..Default::default()
            });
        }
    }

    /// Returns a reference to a mix by name.
    #[must_use]
    pub fn get_mix(&self, name: &str) -> Option<&AudioMix> {
        self.mixes.iter().find(|c| c.name == name)
    }

    /// Returns a mutable reference to a mix by name.
    pub fn get_mix_mut(&mut self, name: &str) -> Option<&mut AudioMix> {
        self.mixes.iter_mut().find(|c| c.name == name)
    }

    /// Returns all mix names.
    #[must_use]
    pub fn get_all_mixes(&self) -> Vec<MixName> {
        self.mixes.iter().map(|m| m.name.clone()).collect()
    }

    /// Adds a channel to a mix if both exist, and it is not already routed.
    pub fn add_channel_to_mix(&mut self, channel_name: &str, mix_name: &str) -> bool {
        let channel_exists = self.channels.iter().any(|c| c.name == channel_name);
        if let (true, Some(mix)) = (channel_exists, self.get_mix_mut(mix_name))
            && !mix.channels.iter().any(|c| c == channel_name)
        {
            mix.channels.push(ChannelName::from(channel_name));
            return true;
        }
        false
    }

    /// Removes a channel from a mix. Returns true if removed.
    pub fn remove_channel_from_mix(&mut self, channel_name: &str, mix_name: &str) -> bool {
        if let Some(mix) = self.get_mix_mut(mix_name) {
            let initial_len = mix.channels.len();
            mix.channels.retain(|c| c != channel_name);
            mix.channels.len() != initial_len
        } else {
            false
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use pipelink_common::types::ChannelName;

    use super::ConfigGraph;

    #[test]
    fn test_add_and_get_channels() {
        let mut graph = ConfigGraph::new();
        graph.add_channel("mic");
        graph.add_channel("game");

        // Attempt duplicate addition
        graph.add_channel("mic");

        assert_eq!(graph.channels.len(), 2);
        assert!(graph.get_channel("mic").is_some());
        assert!(graph.get_channel("game").is_some());
        assert!(graph.get_channel("nonexistent").is_none());
        assert_eq!(graph.get_all_channels(), vec!["mic", "game"]);
    }

    #[test]
    fn test_add_and_get_mixes() {
        let mut graph = ConfigGraph::new();
        graph.add_mix("headphones");
        graph.add_mix("stream");

        // Attempt duplicate addition
        graph.add_mix("headphones");

        assert_eq!(graph.mixes.len(), 2);
        assert!(graph.get_mix("headphones").is_some());
        assert!(graph.get_mix("stream").is_some());
        assert!(graph.get_mix("speakers").is_none());
        assert_eq!(graph.get_all_mixes(), vec!["headphones", "stream"]);
    }

    #[test]
    fn test_channels_on_different_mixes_or_none() {
        let mut graph = ConfigGraph::new();

        graph.add_channel("mic");
        graph.add_channel("game");
        graph.add_channel("unassigned");

        graph.add_mix("headphones");
        graph.add_mix("stream");

        assert!(graph.add_channel_to_mix("mic", "headphones"));
        assert!(graph.add_channel_to_mix("mic", "stream"));
        assert!(graph.add_channel_to_mix("game", "headphones"));

        let headphones = graph.get_mix("headphones").unwrap();
        assert_eq!(headphones.channels, vec!["mic", "game"]);

        let stream = graph.get_mix("stream").unwrap();
        assert_eq!(stream.channels, vec!["mic"]);

        assert!(graph.get_channel("unassigned").is_some());
        assert!(
            !headphones
                .channels
                .contains(&ChannelName::from("unassigned"))
        );
        assert!(!stream.channels.contains(&ChannelName::from("unassigned")));
    }

    #[test]
    fn test_add_channel_to_mix_validation() {
        let mut graph = ConfigGraph::new();
        graph.add_channel("mic");
        graph.add_mix("headphones");

        assert!(!graph.add_channel_to_mix("phantom", "headphones"));
        assert!(!graph.add_channel_to_mix("mic", "phantom_mix"));
        assert!(graph.add_channel_to_mix("mic", "headphones"));
        assert!(!graph.add_channel_to_mix("mic", "headphones"));
        assert_eq!(graph.get_mix("headphones").unwrap().channels.len(), 1);
    }

    #[test]
    fn test_remove_channel_from_mix() {
        let mut graph = ConfigGraph::new();
        graph.add_channel("mic");
        graph.add_mix("headphones");
        graph.add_channel_to_mix("mic", "headphones");

        assert!(graph.remove_channel_from_mix("mic", "headphones"));
        assert_eq!(graph.get_mix("headphones").unwrap().channels.len(), 0);
        assert!(graph.get_channel("mic").is_some());
        assert!(!graph.remove_channel_from_mix("mic", "headphones"));
        assert!(!graph.remove_channel_from_mix("mic", "phantom_mix"));
    }

    #[test]
    fn test_active_mix_and_output() {
        let mut graph = ConfigGraph::new();
        assert_eq!(graph.get_active_mix(), None);
        assert_eq!(graph.get_output(), None);

        graph.set_active_mix("headphones");
        assert_eq!(graph.get_active_mix(), Some("headphones"));

        graph.set_output("alsa_output.pci-0000_00_1f.3");
        assert_eq!(graph.get_output(), Some("alsa_output.pci-0000_00_1f.3"));
    }
}
