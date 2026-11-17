/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Core GUI state types and user/daemon `Message` definitions.

use pipelink_common::{
    events::{GraphSnapshot, StateEvent},
    types::{AppDisplay, ChannelName, MixName},
};

use super::{InstalledAppsPayload, OutputOption};

/// Active top-level sidebar destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum NavPage {
    #[default]
    Mixes,
    Settings,
}

/// Interactive `+ Create channel` UI flow (app pre-selection vs custom empty name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CreateChannelMode {
    AppPicker(String),
    EmptyInput(String),
}

/// Active Channel Detail modal state, optional `+ Add app` query, and rename draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChannelModalState {
    pub(crate) channel: ChannelName,
    pub(crate) rename_draft: String,
    pub(crate) picker_query: Option<String>,
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
    ChannelVolumeChanged {
        channel: ChannelName,
        percent: f32,
    },
    ChannelVolumeReleased {
        channel: ChannelName,
    },
    ToggleChannelMute {
        channel: ChannelName,
        muted: bool,
    },
    MoveChannel {
        channel: ChannelName,
        to_index: usize,
    },
    OpenCreateChannel,
    OpenEmptyChannelInput,
    CancelCreateChannel,
    ChannelCreateSearchChanged(String),
    NewChannelNameChanged(String),
    SubmitCreateChannel,
    CreateChannelForApp {
        channel: ChannelName,
        app: AppDisplay,
    },
    RemoveChannel(ChannelName),
    OpenChannelDetails(ChannelName),
    CloseChannelDetails,
    ChannelRenameDraftChanged(String),
    SubmitRenameChannel,
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
    StartRenameMix(MixName),
    CancelRenameMix,
    MixRenameDraftChanged(String),
    SubmitRenameMix,
    RemoveMix(MixName),
    RefreshSnapshot,
    DismissError,
}
