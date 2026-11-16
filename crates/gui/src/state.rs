/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Core `iced` application state, messages, and update loop.

mod actions;
mod apps;
mod events;
mod output;

use std::{collections::HashMap, time::Instant};

pub(crate) use actions::InstalledAppsPayload;
pub(crate) use apps::{PickerAppEntry, RoutedChannelApp};
use iced::{Element, Task, widget::svg};
pub(crate) use output::OutputOption;
#[cfg(test)]
pub(crate) use output::normalize_output_name;
use pipelink_common::{
    events::{GraphSnapshot, InstalledAppInfo, MixRouteInfo, StateEvent},
    types::{AppDisplay, ChannelName, MixName},
};
use pipelink_ipc::UnixSocketClient;

/// Active top-level sidebar destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum NavPage {
    #[default]
    Mixes,
    Settings,
}

/// Active Channel Detail modal state and optional `+ Add app` search query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChannelModalState {
    pub(crate) channel: ChannelName,
    pub(crate) picker_query: Option<String>,
}

/// Root GUI state synchronized with the `pipelink` daemon via IPC.
pub(crate) struct App {
    pub(crate) client: UnixSocketClient,
    pub(crate) connected: bool,
    pub(crate) page: NavPage,
    pub(crate) snapshot: GraphSnapshot,
    pub(crate) installed_apps: Vec<InstalledAppInfo>,
    pub(crate) icon_handles: HashMap<String, svg::Handle>,
    pub(crate) last_error: Option<String>,
    pub(crate) creating_channel: bool,
    pub(crate) new_channel_name: String,
    pub(crate) creating_mix: bool,
    pub(crate) new_mix_name: String,
    pub(crate) modal: Option<ChannelModalState>,
    pub(crate) gain_throttle: HashMap<(ChannelName, MixName), Instant>,
}

/// Messages produced by user interactions and background IPC tasks.
#[derive(Debug, Clone)]
pub(crate) enum Message {
    DaemonConnected,
    DaemonDisconnected,
    SnapshotLoaded(Result<GraphSnapshot, String>),
    InstalledAppsLoaded(Result<InstalledAppsPayload, String>),
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
    OpenChannelDetails(ChannelName),
    CloseChannelDetails,
    ToggleAppPicker,
    AppSearchChanged(String),
    RouteApp {
        app: AppDisplay,
        channel: ChannelName,
    },
    UnrouteApp(AppDisplay),
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
            icon_handles: HashMap::new(),
            last_error: None,
            creating_channel: false,
            new_channel_name: String::new(),
            creating_mix: false,
            new_mix_name: String::new(),
            modal: None,
            gain_throttle: HashMap::new(),
        };

        (state, initial)
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DaemonConnected | Message::RefreshSnapshot => {
                if matches!(message, Message::DaemonConnected) {
                    self.connected = true;
                    self.last_error = None;
                }
                return Task::batch([
                    actions::fetch_snapshot(self.client.clone()),
                    actions::fetch_installed_apps(self.client.clone()),
                ]);
            },
            Message::DaemonDisconnected => self.connected = false,
            Message::SnapshotLoaded(Ok(snapshot)) => {
                self.connected = true;
                self.snapshot = snapshot;
            },
            Message::SnapshotLoaded(Err(err)) | Message::MutationFinished(Err(err)) => {
                self.last_error = Some(err);
            },
            Message::InstalledAppsLoaded(Ok(payload)) => {
                self.installed_apps = payload.apps;
                self.icon_handles = payload.icons;
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
            } => return actions::toggle_route(self.client.clone(), channel, mix, active),
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
            } => return actions::set_mute(self.client.clone(), channel, mix, muted),
            Message::RemoveChannel(name) => {
                return actions::remove_channel(self.client.clone(), name);
            },
            Message::RemoveMix(name) => return actions::remove_mix(self.client.clone(), name),
            Message::DismissError => self.last_error = None,
            other => return self.update_overlays_and_forms(other),
        }

        Task::none()
    }

    fn update_overlays_and_forms(&mut self, message: Message) -> Task<Message> {
        match message {
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
            Message::OpenChannelDetails(channel) => {
                self.modal = Some(ChannelModalState {
                    channel,
                    picker_query: None,
                });
                return actions::fetch_installed_apps(self.client.clone());
            },
            Message::CloseChannelDetails => self.modal = None,
            Message::ToggleAppPicker => {
                if let Some(modal) = &mut self.modal {
                    modal.picker_query = modal.picker_query.is_none().then(String::new);
                }
            },
            Message::AppSearchChanged(query) => {
                if let Some(modal) = &mut self.modal {
                    modal.picker_query = Some(query);
                }
            },
            Message::RouteApp { app, channel } => {
                if let Some(modal) = &mut self.modal {
                    modal.picker_query = None;
                }
                return actions::route_app(self.client.clone(), app, channel);
            },
            Message::UnrouteApp(app) => {
                return actions::unroute_app(self.client.clone(), app);
            },
            _ => {},
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
