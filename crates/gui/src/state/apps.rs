/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Channel-assigned application projections and `+ Add app` search filtering.

use std::collections::HashSet;

use iced::widget::svg;
use pipelink_common::types::{AppDisplay, ChannelName};

use super::App;

/// View model for an application routed to the currently inspected channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RoutedChannelApp {
    /// Configured application display name stored in `app_routes`.
    pub(crate) display_name: AppDisplay,
    /// Optional lookup key into `App::app_icons`.
    pub(crate) icon_key: Option<String>,
    /// Active stream count when currently running, or `None` if offline.
    pub(crate) active_streams: Option<usize>,
}

/// View model for an entry inside the searchable `+ Add app` picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PickerAppEntry {
    /// Human-readable label sent as `RouteApp { app, .. }`.
    pub(crate) display_name: AppDisplay,
    /// Optional lookup key into `App::app_icons`.
    pub(crate) icon_key: Option<String>,
    /// Currently assigned channel, if routed anywhere.
    pub(crate) current_channel: Option<ChannelName>,
    /// Active stream count when running, or `None` for installed-only apps.
    pub(crate) active_streams: Option<usize>,
}

/// Active and installed application groups filtered for the `+ Add app` picker.
#[derive(Debug, Default)]
pub(crate) struct PickerGroups {
    /// Currently running `PipeWire` audio stream applications.
    pub(crate) active: Vec<PickerAppEntry>,
    /// Installed `.desktop` applications not already present in `active`.
    pub(crate) installed: Vec<PickerAppEntry>,
}

impl App {
    /// Looks up a pre-resolved icon asset for a routed or picker application row.
    pub(crate) fn lookup_app_icon(
        &self,
        display_name: &str,
        icon_key: Option<&str>,
    ) -> Option<&svg::Handle> {
        if let Some(key) = icon_key
            && let Some(found) = self.icon_handles.get(&key.to_ascii_lowercase())
        {
            return Some(found);
        }
        self.icon_handles.get(&display_name.to_ascii_lowercase())
    }

    /// Returns all applications routed to `channel`, annotated with live stream counts
    /// and matching `.desktop` icon lookup keys when available.
    pub(crate) fn routed_apps_for_channel(&self, channel: &ChannelName) -> Vec<RoutedChannelApp> {
        let mut routed: Vec<RoutedChannelApp> = self
            .snapshot
            .app_routes
            .iter()
            .filter(|(_, ch)| ch.eq_ignore_ascii_case(channel))
            .map(|(app, _)| {
                let matched = self.snapshot.apps.iter().find(|a| {
                    a.display_name.eq_ignore_ascii_case(app) || a.app_name.eq_ignore_ascii_case(app)
                });
                let icon_key = self.find_icon_key(app, matched.map(|m| m.app_name.as_str()));

                RoutedChannelApp {
                    display_name: app.clone(),
                    icon_key,
                    active_streams: matched.map(|a| a.stream_count),
                }
            })
            .collect();

        routed.sort_by_key(|a| a.display_name.to_ascii_lowercase());
        routed
    }

    /// Builds filtered `Active` and `Installed` application groups for `+ Add app`
    /// (excluding apps already assigned to `exclude_channel` when provided).
    pub(crate) fn picker_apps(
        &self,
        exclude_channel: Option<&ChannelName>,
        query: &str,
    ) -> PickerGroups {
        let needle = query.trim().to_ascii_lowercase();
        let mut seen = HashSet::new();
        let mut active = Vec::new();

        for app in &self.snapshot.apps {
            let key = app.display_name.to_ascii_lowercase();
            let slug = app.app_name.to_ascii_lowercase();
            seen.insert(key.clone());
            seen.insert(slug.clone());

            if exclude_channel.is_some_and(|ch| {
                app.channel
                    .as_ref()
                    .is_some_and(|cur| cur.eq_ignore_ascii_case(ch))
            }) {
                continue;
            }
            if !needle.is_empty() && !key.contains(&needle) && !slug.contains(&needle) {
                continue;
            }

            active.push(PickerAppEntry {
                display_name: app.display_name.clone(),
                icon_key: self.find_icon_key(&app.display_name, Some(&app.app_name)),
                current_channel: app.channel.clone(),
                active_streams: Some(app.stream_count),
            });
        }
        active.sort_by_key(|a| a.display_name.to_ascii_lowercase());

        let mut installed = Vec::new();
        for entry in &self.installed_apps {
            let key = entry.name.to_ascii_lowercase();
            let id_key = entry.id.to_ascii_lowercase();
            let bin_key = entry
                .exec_bin
                .as_deref()
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();

            if seen.contains(&key)
                || seen.contains(&id_key)
                || (!bin_key.is_empty() && seen.contains(&bin_key))
            {
                continue;
            }

            let current_channel = self.find_route_for_name(&entry.name);
            if exclude_channel.is_some_and(|ch| {
                current_channel
                    .as_ref()
                    .is_some_and(|cur| cur.eq_ignore_ascii_case(ch))
            }) {
                continue;
            }
            if !needle.is_empty()
                && !key.contains(&needle)
                && !id_key.contains(&needle)
                && !bin_key.contains(&needle)
            {
                continue;
            }

            seen.insert(key);
            let icon_key = entry
                .icon
                .clone()
                .or_else(|| entry.exec_bin.clone())
                .or_else(|| Some(entry.id.to_string()));
            installed.push(PickerAppEntry {
                display_name: AppDisplay::from(entry.name.as_str()),
                icon_key,
                current_channel,
                active_streams: None,
            });
        }

        PickerGroups { active, installed }
    }

    fn find_route_for_name(&self, name: &str) -> Option<ChannelName> {
        self.snapshot
            .app_routes
            .iter()
            .find(|(app, _)| app.eq_ignore_ascii_case(name))
            .map(|(_, ch)| ch.clone())
    }

    fn find_icon_key(&self, app: &str, slug: Option<&str>) -> Option<String> {
        let matched = self.installed_apps.iter().find(|entry| {
            entry.name.eq_ignore_ascii_case(app)
                || entry.id.eq_ignore_ascii_case(app)
                || entry
                    .exec_bin
                    .as_deref()
                    .is_some_and(|bin| bin.eq_ignore_ascii_case(app))
                || slug.is_some_and(|s| {
                    entry.name.eq_ignore_ascii_case(s)
                        || entry.id.eq_ignore_ascii_case(s)
                        || entry
                            .exec_bin
                            .as_deref()
                            .is_some_and(|bin| bin.eq_ignore_ascii_case(s))
                })
        });

        matched
            .and_then(|entry| {
                entry
                    .icon
                    .clone()
                    .or_else(|| entry.exec_bin.clone())
                    .or_else(|| Some(entry.id.to_string()))
            })
            .or_else(|| slug.map(ToString::to_string))
    }
}
