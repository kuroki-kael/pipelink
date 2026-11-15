/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Audio routing topology models and configuration graph.

use pipelink_common::types::{AppDisplay, ChannelName, MixName, NodeName};
use serde::{Deserialize, Serialize};

/// A virtual Sink an app or microphone gets routed to.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub struct AudioChannel {
    /// Channel name.
    pub name: ChannelName,
    /// Application display names routed to this channel.
    #[serde(default)]
    pub apps: Vec<AppDisplay>,
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

    /// Ensures a channel entry exists in [`ConfigGraph::channels`].
    pub fn ensure_channel(&mut self, name: &ChannelName) {
        if !self.channels.iter().any(|ch| &ch.name == name) {
            self.channels.push(AudioChannel {
                name: name.clone(),
                ..Default::default()
            });
        }
    }

    /// Removes `name` and all application routing rules assigned to it.
    pub fn remove_channel(&mut self, name: &ChannelName) {
        self.channels.retain(|ch| &ch.name != name);
    }

    /// Assigns `app` to `channel`, removing any prior assignment of `app`.
    ///
    /// Matching is case-insensitive so display-name lookups normalize cleanly.
    /// Returns `false` if `channel` does not exist in the config graph.
    pub fn set_app_route(&mut self, app: AppDisplay, channel: &ChannelName) -> bool {
        if !self.channels.iter().any(|ch| &ch.name == channel) {
            return false;
        }

        self.remove_app_route(&app);
        if let Some(target) = self.channels.iter_mut().find(|ch| &ch.name == channel) {
            target.apps.push(app);
            return true;
        }
        false
    }

    /// Removes `app` (case-insensitively) from whichever channel owns it.
    ///
    /// Returns the stored [`AppDisplay`] name if a rule existed.
    pub fn remove_app_route(&mut self, app: &AppDisplay) -> Option<AppDisplay> {
        let mut removed = None;
        for ch in &mut self.channels {
            if let Some(pos) = ch.apps.iter().position(|a| a.eq_ignore_ascii_case(app)) {
                let entry = ch.apps.remove(pos);
                if removed.is_none() {
                    removed = Some(entry);
                }
            }
        }
        removed
    }

    /// Returns the configured destination channel for `app` (case-insensitive), if any.
    #[must_use]
    pub fn find_app_channel(&self, app: &str) -> Option<&ChannelName> {
        self.channels
            .iter()
            .find(|ch| ch.apps.iter().any(|a| a.eq_ignore_ascii_case(app)))
            .map(|ch| &ch.name)
    }

    /// Returns all configured `(app, channel)` pairs in channel order.
    #[must_use]
    pub fn app_routes(&self) -> Vec<(AppDisplay, ChannelName)> {
        self.channels
            .iter()
            .flat_map(|ch| ch.apps.iter().map(|app| (app.clone(), ch.name.clone())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_graph_app_route_lifecycle() {
        let mut cfg = ConfigGraph::new();
        let game = ChannelName::from("Game");
        let chat = ChannelName::from("Chat");

        assert!(!cfg.set_app_route(AppDisplay::from("Discord"), &chat));

        cfg.ensure_channel(&game);
        cfg.ensure_channel(&chat);

        assert!(cfg.set_app_route(AppDisplay::from("Discord"), &chat));
        assert_eq!(cfg.find_app_channel("discord"), Some(&chat));
        assert_eq!(
            cfg.app_routes(),
            vec![(AppDisplay::from("Discord"), chat.clone())]
        );

        // Moving the app to another channel replaces the previous rule without
        // duplicates.
        assert!(cfg.set_app_route(AppDisplay::from("discord"), &game));
        assert_eq!(cfg.find_app_channel("Discord"), Some(&game));
        assert_eq!(
            cfg.app_routes(),
            vec![(AppDisplay::from("discord"), game.clone())]
        );

        // Deleting a channel clears its app rules as well.
        cfg.remove_channel(&game);
        assert_eq!(cfg.find_app_channel("Discord"), None);
        assert_eq!(cfg.app_routes(), Vec::new());
    }
}
