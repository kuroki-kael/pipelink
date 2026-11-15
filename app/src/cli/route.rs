/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The route commands of the cli.

use clap::Subcommand;
use pipelink_common::{
    events::{ControlEvent, ControlReply, StateEvent},
    types::{ChannelName, MixName},
};

use super::fail;

/// The channel-to-mix routes and route gain/mute to manage.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum RouteAction {
    /// List active channel-to-mix routes
    List,
    /// Route a channel into a mix
    Set { channel: String, mix: String },
    /// Remove a channel-to-mix route
    Remove { channel: String, mix: String },
    /// Set route gain in percent (e.g. 100 = unity, 50 = half)
    Gain {
        channel: String,
        mix: String,
        #[arg(allow_hyphen_values = true)]
        percent: f32,
    },
    /// Mute a channel in a mix
    Mute { channel: String, mix: String },
    /// Unmute a channel in a mix
    Unmute { channel: String, mix: String },
}

/// Runs a route command against the daemon.
pub(super) async fn run(action: RouteAction) {
    match action {
        RouteAction::List => match crate::ipc::send_query(ControlEvent::ListMixChannels).await {
            Ok(ControlReply::MixChannels { channels }) => {
                if channels.is_empty() {
                    println!("no mixes configured");
                }
                for (mix, mix_channels) in channels {
                    if mix_channels.is_empty() {
                        println!("{mix}: (none)");
                    } else {
                        let list: Vec<&str> = mix_channels.iter().map(AsRef::as_ref).collect();
                        println!("{mix}: {}", list.join(", "));
                    }
                }
            },
            Ok(other) => fail(format!("unexpected answer: {other:?}")),
            Err(e) => fail(e),
        },
        RouteAction::Set { channel, mix } => {
            let ch = ChannelName::from(channel.as_str());
            let mx = MixName::from(mix.as_str());
            let req = ControlEvent::AddChannelMix {
                channel: ch.clone(),
                mix: mx.clone(),
            };
            let reply = send_route_mutation(req, &ch, &mx, |event| {
                matches!(
                    event,
                    StateEvent::MixChannelAdded { channel: c, mix: m }
                        if c.eq_ignore_ascii_case(&ch) && m.eq_ignore_ascii_case(&mx)
                )
            })
            .await;
            if let StateEvent::MixChannelAdded { channel: c, mix: m } = reply {
                println!("Routed '{c}' -> '{m}'");
            }
        },
        RouteAction::Remove { channel, mix } => {
            let ch = ChannelName::from(channel.as_str());
            let mx = MixName::from(mix.as_str());
            let req = ControlEvent::RemoveChannelMix {
                channel: ch.clone(),
                mix: mx.clone(),
            };
            let reply = send_route_mutation(req, &ch, &mx, |event| {
                matches!(
                    event,
                    StateEvent::MixChannelRemoved { channel: c, mix: m }
                        if c.eq_ignore_ascii_case(&ch) && m.eq_ignore_ascii_case(&mx)
                )
            })
            .await;
            if let StateEvent::MixChannelRemoved { channel: c, mix: m } = reply {
                println!("Removed route '{c}' -> '{m}'");
            }
        },
        RouteAction::Gain {
            channel,
            mix,
            percent,
        } => {
            let ch = ChannelName::from(channel.as_str());
            let mx = MixName::from(mix.as_str());
            let req = ControlEvent::SetChannelGain {
                channel: ch.clone(),
                mix: mx.clone(),
                percent,
            };
            let reply = send_route_mutation(req, &ch, &mx, |event| {
                matches!(
                    event,
                    StateEvent::GainChanged { channel: c, mix: m, .. }
                        if c.eq_ignore_ascii_case(&ch) && m.eq_ignore_ascii_case(&mx)
                )
            })
            .await;
            if let StateEvent::GainChanged {
                channel: c,
                mix: m,
                gain_linear,
                percent,
            } = reply
            {
                println!("Gain for '{c}' -> '{m}' set to {percent}% ({gain_linear}x)");
            }
        },
        RouteAction::Mute { channel, mix } => set_mute(channel, mix, true).await,
        RouteAction::Unmute { channel, mix } => set_mute(channel, mix, false).await,
    }
}

async fn set_mute(channel: String, mix: String, muted: bool) {
    let ch = ChannelName::from(channel.as_str());
    let mx = MixName::from(mix.as_str());
    let req = ControlEvent::SetChannelMute {
        channel: ch.clone(),
        mix: mx.clone(),
        muted,
    };
    let reply = send_route_mutation(req, &ch, &mx, |event| {
        matches!(
            event,
            StateEvent::MuteChanged { channel: c, mix: m, .. }
                if c.eq_ignore_ascii_case(&ch) && m.eq_ignore_ascii_case(&mx)
        )
    })
    .await;
    let state = if muted { "muted" } else { "unmuted" };
    if let StateEvent::MuteChanged {
        channel: c, mix: m, ..
    } = reply
    {
        println!("Route '{c}' -> '{m}' {state}");
    }
}

async fn send_route_mutation(
    request: ControlEvent,
    ch: &ChannelName,
    mx: &MixName,
    is_success: impl Fn(&StateEvent) -> bool,
) -> StateEvent {
    let result = crate::ipc::send_mutation(request, |event| {
        is_success(event)
            || matches!(
                event,
                StateEvent::RouteOperationFailed { channel, mix, .. }
                    if channel.eq_ignore_ascii_case(ch) && mix.eq_ignore_ascii_case(mx)
            )
    })
    .await;

    match result {
        Ok(StateEvent::RouteOperationFailed { error, .. }) => {
            fail(format!("route '{ch}' -> '{mx}' failed: {error}"));
        },
        Ok(event) => event,
        Err(e) => fail(e),
    }
}
