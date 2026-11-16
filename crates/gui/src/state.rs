/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Core `iced` application state, messages, and update loop.

mod actions;
mod events;

use std::{collections::HashMap, fmt, time::Instant};

use iced::{Element, Task};
use pipelink_common::{
    events::{GraphSnapshot, InstalledAppInfo, MixRouteInfo, OutputDeviceInfo, StateEvent},
    types::{ChannelName, MixName},
};
use pipelink_ipc::UnixSocketClient;

/// Active top-level sidebar destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum NavPage {
    #[default]
    Mixes,
    Settings,
}

/// Wrapper allowing [`OutputDeviceInfo`] to render cleanly inside an
/// `iced::widget::pick_list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputOption(pub(crate) OutputDeviceInfo);

impl fmt::Display for OutputOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&normalize_output_name(self.0.display_name()))
    }
}

/// Normalizes verbose ALSA/`PipeWire` sink descriptions into compact human-readable
/// labels.
pub(crate) fn normalize_output_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Unknown output".to_string();
    }

    let cleaned = if !trimmed.contains(' ')
        && (trimmed.starts_with("alsa_output.") || trimmed.starts_with("alsa_input."))
    {
        let without_dir = trimmed
            .strip_prefix("alsa_output.")
            .or_else(|| trimmed.strip_prefix("alsa_input."))
            .unwrap_or(trimmed);
        let without_bus = without_dir
            .strip_prefix("usb-")
            .or_else(|| without_dir.strip_prefix("pci-"))
            .unwrap_or(without_dir);
        let base = without_bus.split('.').next().unwrap_or(without_bus);
        let base = base.strip_suffix("-00").unwrap_or(base);
        base.replace('_', " ")
    } else {
        trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
    };

    for suffix in [
        " Digital Stereo (IEC958)",
        " Analog Stereo",
        " Digital Stereo",
        " Pro Audio",
        " Stereo",
    ] {
        if let Some(prefix) = cleaned.strip_suffix(suffix) {
            let candidate = prefix.trim();
            if !candidate.is_empty() {
                return candidate.to_string();
            }
        }
    }

    cleaned
}

/// Root GUI state synchronized with the `pipelink` daemon via IPC.
pub(crate) struct App {
    pub(crate) client: UnixSocketClient,
    pub(crate) connected: bool,
    pub(crate) page: NavPage,
    pub(crate) snapshot: GraphSnapshot,
    pub(crate) installed_apps: Vec<InstalledAppInfo>,
    pub(crate) last_error: Option<String>,
    pub(crate) creating_channel: bool,
    pub(crate) new_channel_name: String,
    pub(crate) creating_mix: bool,
    pub(crate) new_mix_name: String,
    pub(crate) gain_throttle: HashMap<(ChannelName, MixName), Instant>,
}

/// Messages produced by user interactions and background IPC tasks.
#[derive(Debug, Clone)]
pub(crate) enum Message {
    DaemonConnected,
    DaemonDisconnected,
    SnapshotLoaded(Result<GraphSnapshot, String>),
    InstalledAppsLoaded(Result<Vec<InstalledAppInfo>, String>),
    StateEventReceived(StateEvent),
    MutationFinished(Result<(), String>),
    SelectPage(NavPage),
    SelectOutput(OutputOption),
    SetDefaultSink(String),
    SelectActiveMix(MixName),
    ToggleRoute {
        channel: ChannelName,
        mix: MixName,
        active: bool,
    },
    GainChanged {
        channel: ChannelName,
        mix: MixName,
        percent: f32,
    },
    GainReleased {
        channel: ChannelName,
        mix: MixName,
    },
    ToggleMute {
        channel: ChannelName,
        mix: MixName,
        muted: bool,
    },
    OpenCreateChannel,
    CancelCreateChannel,
    NewChannelNameChanged(String),
    SubmitCreateChannel,
    RemoveChannel(ChannelName),
    OpenCreateMix,
    CancelCreateMix,
    NewMixNameChanged(String),
    SubmitCreateMix,
    RemoveMix(MixName),
    RefreshSnapshot,
    DismissError,
}

impl App {
    pub(crate) fn boot() -> (Self, Task<Message>) {
        let client = UnixSocketClient::new();
        let initial = Task::batch([
            actions::fetch_snapshot(client.clone()),
            actions::fetch_installed_apps(client.clone()),
        ]);

        let state = Self {
            client,
            connected: false,
            page: NavPage::Mixes,
            snapshot: GraphSnapshot::default(),
            installed_apps: Vec::new(),
            last_error: None,
            creating_channel: false,
            new_channel_name: String::new(),
            creating_mix: false,
            new_mix_name: String::new(),
            gain_throttle: HashMap::new(),
        };

        (state, initial)
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DaemonConnected => {
                self.connected = true;
                self.last_error = None;
                return Task::batch([
                    actions::fetch_snapshot(self.client.clone()),
                    actions::fetch_installed_apps(self.client.clone()),
                ]);
            },
            Message::DaemonDisconnected => {
                self.connected = false;
            },
            Message::SnapshotLoaded(Ok(snapshot)) => {
                self.connected = true;
                self.snapshot = snapshot;
            },
            Message::SnapshotLoaded(Err(err)) | Message::MutationFinished(Err(err)) => {
                self.last_error = Some(err);
            },
            Message::InstalledAppsLoaded(Ok(apps)) => {
                self.installed_apps = apps;
            },
            Message::InstalledAppsLoaded(Err(_)) | Message::MutationFinished(Ok(())) => {},
            Message::StateEventReceived(event) => {
                if self.apply_state_event(event) {
                    return actions::fetch_snapshot(self.client.clone());
                }
            },
            Message::SelectPage(page) => self.page = page,
            Message::SelectOutput(option) => {
                return actions::select_output(self.client.clone(), option.0.name.to_string());
            },
            Message::SetDefaultSink(target) => {
                return actions::set_default_sink(self.client.clone(), target);
            },
            Message::SelectActiveMix(mix) => {
                return actions::select_active_mix(self.client.clone(), mix);
            },
            Message::ToggleRoute {
                channel,
                mix,
                active,
            } => {
                return actions::toggle_route(self.client.clone(), channel, mix, active);
            },
            Message::GainChanged {
                channel,
                mix,
                percent,
            } => return self.handle_gain_changed(channel, mix, percent),
            Message::GainReleased { channel, mix } => {
                return self.handle_gain_released(channel, mix);
            },
            Message::ToggleMute {
                channel,
                mix,
                muted,
            } => {
                return actions::set_mute(self.client.clone(), channel, mix, muted);
            },
            Message::OpenCreateChannel => {
                self.creating_channel = true;
                self.new_channel_name.clear();
            },
            Message::CancelCreateChannel => {
                self.creating_channel = false;
                self.new_channel_name.clear();
            },
            Message::NewChannelNameChanged(name) => self.new_channel_name = name,
            Message::SubmitCreateChannel => return self.submit_create_channel(),
            Message::RemoveChannel(name) => {
                return actions::remove_channel(self.client.clone(), name);
            },
            Message::OpenCreateMix => {
                self.creating_mix = true;
                self.new_mix_name.clear();
            },
            Message::CancelCreateMix => {
                self.creating_mix = false;
                self.new_mix_name.clear();
            },
            Message::NewMixNameChanged(name) => self.new_mix_name = name,
            Message::SubmitCreateMix => return self.submit_create_mix(),
            Message::RemoveMix(name) => return actions::remove_mix(self.client.clone(), name),
            Message::RefreshSnapshot => {
                return Task::batch([
                    actions::fetch_snapshot(self.client.clone()),
                    actions::fetch_installed_apps(self.client.clone()),
                ]);
            },
            Message::DismissError => self.last_error = None,
        }

        Task::none()
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        self.render_window()
    }

    /// Finds the current route cell for `(channel, mix)` in the snapshot.
    pub(crate) fn find_route(&self, channel: &ChannelName, mix: &MixName) -> Option<&MixRouteInfo> {
        self.snapshot
            .routes
            .iter()
            .find(|r| r.channel == *channel && r.mix == *mix)
    }

    /// Returns whether `channel` is currently `WirePlumber`'s default audio sink.
    pub(crate) fn is_default_channel(&self, channel: &ChannelName) -> bool {
        let Some(default_sink) = &self.snapshot.default_sink else {
            return false;
        };
        let expected_node = format!("pipelink.channel.{channel}");
        default_sink.eq_ignore_ascii_case(&expected_node)
            || default_sink.eq_ignore_ascii_case(channel)
    }

    /// Returns the number of configured application routes targeting `channel`.
    pub(crate) fn routed_app_count(&self, channel: &ChannelName) -> usize {
        self.snapshot
            .app_routes
            .iter()
            .filter(|(_, ch)| ch.eq_ignore_ascii_case(channel))
            .count()
    }
}
