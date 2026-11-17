/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Audio routing topology models and configuration graph.

use pipelink_common::types::{AppDisplay, ChannelName, MixName, NodeName};
use serde::{Deserialize, Serialize};

/// A virtual sink that applications get routed to.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub struct AudioChannel {
    /// Channel name.
    pub name: ChannelName,
    /// Application display names routed to this channel.
    #[serde(default)]
    pub apps: Vec<AppDisplay>,
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
    /// Name of mix that goes to output (runtime fallback; stored in `RuntimeState`).
    #[serde(default, skip_serializing)]
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

    /// Returns the starter topology created on first daemon launch when no `config.json`
    /// exists yet.
    #[must_use]
    pub fn starter_default() -> Self {
        let mut cfg = Self::new();
        for ch in ["System", "Music", "Games", "Voice"] {
            cfg.ensure_channel(&ChannelName::from(ch));
        }
        let personal = MixName::from("Personal");
        let stream = MixName::from("Stream");
        cfg.ensure_mix(&personal);
        cfg.ensure_mix(&stream);
        for ch in ["System", "Music", "Games", "Voice"] {
            cfg.set_channel_mix(&ChannelName::from(ch), &personal);
        }
        cfg
    }

    /// Ensures a channel entry exists in [`ConfigGraph::channels`] (case-insensitive).
    pub fn ensure_channel(&mut self, name: &ChannelName) {
        if !self
            .channels
            .iter()
            .any(|ch| ch.name.eq_ignore_ascii_case(name))
        {
            self.channels.push(AudioChannel {
                name: name.clone(),
                ..Default::default()
            });
        }
    }

    /// Removes `name` (case-insensitive), its app rules, and its references in mixes.
    pub fn remove_channel(&mut self, name: &ChannelName) -> bool {
        let before = self.channels.len();
        self.channels
            .retain(|ch| !ch.name.eq_ignore_ascii_case(name));
        let removed_channel = self.channels.len() != before;
        let mut removed_from_mix = false;
        for mix in &mut self.mixes {
            let mix_before = mix.channels.len();
            mix.channels.retain(|ch| !ch.eq_ignore_ascii_case(name));
            if mix.channels.len() != mix_before {
                removed_from_mix = true;
            }
        }
        removed_channel || removed_from_mix
    }

    /// Reorders `name` (case-insensitive) to `to_index`, returning `true` if the list
    /// changed.
    pub fn move_channel(&mut self, name: &ChannelName, to_index: usize) -> bool {
        let Some(from_idx) = self
            .channels
            .iter()
            .position(|ch| ch.name.eq_ignore_ascii_case(name))
        else {
            return false;
        };
        let Some(last_idx) = self.channels.len().checked_sub(1) else {
            return false;
        };
        let actual_to = to_index.min(last_idx);
        if from_idx == actual_to {
            return false;
        }
        let ch = self.channels.remove(from_idx);
        self.channels.insert(actual_to, ch);
        true
    }

    /// Ensures a mix entry exists in [`ConfigGraph::mixes`] (case-insensitive).
    pub fn ensure_mix(&mut self, name: &MixName) {
        if !self.mixes.iter().any(|m| m.name.eq_ignore_ascii_case(name)) {
            self.mixes.push(AudioMix {
                name: name.clone(),
                ..Default::default()
            });
        }
    }

    /// Removes `name` (case-insensitive) from [`ConfigGraph::mixes`].
    pub fn remove_mix(&mut self, name: &MixName) -> bool {
        let before = self.mixes.len();
        self.mixes.retain(|m| !m.name.eq_ignore_ascii_case(name));
        self.mixes.len() != before
    }

    /// Records an active `channel -> mix` route in [`ConfigGraph::mixes`].
    pub fn set_channel_mix(&mut self, channel: &ChannelName, mix: &MixName) {
        self.ensure_channel(channel);
        self.ensure_mix(mix);

        let canonical_ch = self
            .channels
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(channel))
            .map_or_else(|| channel.clone(), |c| c.name.clone());

        if let Some(target) = self
            .mixes
            .iter_mut()
            .find(|m| m.name.eq_ignore_ascii_case(mix))
            && !target
                .channels
                .iter()
                .any(|c| c.eq_ignore_ascii_case(&canonical_ch))
        {
            target.channels.push(canonical_ch);
        }
    }

    /// Removes a `channel -> mix` route from [`ConfigGraph::mixes`].
    pub fn remove_channel_mix(&mut self, channel: &ChannelName, mix: &MixName) {
        if let Some(target) = self
            .mixes
            .iter_mut()
            .find(|m| m.name.eq_ignore_ascii_case(mix))
        {
            target.channels.retain(|c| !c.eq_ignore_ascii_case(channel));
        }
    }

    /// Assigns `app` to `channel`, removing any prior assignment of `app`.
    ///
    /// Matching is case-insensitive so display-name lookups normalize cleanly.
    /// Returns `false` if `channel` does not exist in the config graph.
    pub fn set_app_route(&mut self, app: AppDisplay, channel: &ChannelName) -> bool {
        if !self
            .channels
            .iter()
            .any(|ch| ch.name.eq_ignore_ascii_case(channel))
        {
            return false;
        }

        self.remove_app_route(&app);
        if let Some(target) = self
            .channels
            .iter_mut()
            .find(|ch| ch.name.eq_ignore_ascii_case(channel))
        {
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
mod tests;
