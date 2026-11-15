/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The application routing commands of the cli.

use clap::Subcommand;
use pipelink_common::{
    events::{ControlEvent, ControlReply, StateEvent},
    types::{AppDisplay, ChannelName},
};

use super::fail;

/// Application discovery and application-to-channel routing actions.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum AppAction {
    /// List active applications and configured app-to-channel routes
    List,
    /// Route an application (by display name) into a channel
    #[command(visible_alias = "set")]
    Route { app: String, channel: String },
    /// Remove the routing rule for an application
    #[command(visible_alias = "remove")]
    Unroute { app: String },
}

/// Runs an application command against the daemon.
pub(super) async fn run(action: AppAction) {
    match action {
        AppAction::List => {
            let routes = match crate::ipc::send_query(ControlEvent::ListAppChannels).await {
                Ok(ControlReply::AppChannels { routes }) => routes,
                Ok(other) => fail(format!("unexpected answer: {other:?}")),
                Err(e) => fail(e),
            };

            let apps = match crate::ipc::send_query(ControlEvent::ListApps).await {
                Ok(ControlReply::Apps { apps }) => apps,
                Ok(other) => fail(format!("unexpected answer: {other:?}")),
                Err(e) => fail(e),
            };

            println!("Configured app routes:");
            if routes.is_empty() {
                println!("  (none)");
            } else {
                for (app, channel) in routes {
                    println!("  {app} -> {channel}");
                }
            }

            println!("Active apps:");
            if apps.is_empty() {
                println!("  (none)");
            } else {
                for app in apps {
                    let target = app.channel.as_deref().unwrap_or("(default)");
                    println!(
                        "  {} [{}] ({} stream{}) -> {}",
                        app.display_name,
                        app.app_name,
                        app.stream_count,
                        if app.stream_count == 1 { "" } else { "s" },
                        target
                    );
                }
            }
        },
        AppAction::Route { app, channel } => {
            let app_display = AppDisplay::from(app.as_str());
            let ch = ChannelName::from(channel.as_str());
            let req = ControlEvent::RouteApp {
                app: app_display.clone(),
                channel: ch.clone(),
            };
            let reply = crate::ipc::send_mutation(req, |event| match event {
                StateEvent::AppRouted { app: a, channel: c } => {
                    c == &ch && a.eq_ignore_ascii_case(app_display.trim())
                },
                StateEvent::AppRoutingFailed { app: a, .. } => {
                    a.eq_ignore_ascii_case(app_display.trim()) || a == &app_display
                },
                _ => false,
            })
            .await;

            match reply {
                Ok(StateEvent::AppRoutingFailed { error, .. }) => {
                    fail(format!(
                        "failed to route app '{app}' -> '{channel}': {error}"
                    ));
                },
                Ok(StateEvent::AppRouted {
                    app: resolved,
                    channel: ch,
                }) => {
                    println!("Routed app '{resolved}' -> '{ch}'");
                },
                Ok(other) => fail(format!("unexpected state event: {other:?}")),
                Err(e) => fail(e),
            }
        },
        AppAction::Unroute { app } => {
            let app_display = AppDisplay::from(app.as_str());
            let req = ControlEvent::UnrouteApp {
                app: app_display.clone(),
            };
            let reply = crate::ipc::send_mutation(req, |event| match event {
                StateEvent::AppUnrouted { app: a }
                | StateEvent::AppRoutingFailed { app: a, .. } => {
                    a.eq_ignore_ascii_case(app_display.trim()) || a == &app_display
                },
                _ => false,
            })
            .await;

            match reply {
                Ok(StateEvent::AppRoutingFailed { error, .. }) => {
                    fail(format!("failed to remove app route '{app}': {error}"));
                },
                Ok(StateEvent::AppUnrouted { app: resolved }) => {
                    println!("Removed app route for '{resolved}'");
                },
                Ok(other) => fail(format!("unexpected state event: {other:?}")),
                Err(e) => fail(e),
            }
        },
    }
}
