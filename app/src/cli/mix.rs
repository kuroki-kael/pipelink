/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The mix commands of the cli.

use clap::Subcommand;
use pipelink_common::{
    events::{ControlEvent, ControlReply, StateEvent},
    types::MixName,
};

use super::fail;

/// The mixes to manage.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum MixAction {
    /// List all configured mixes
    List,
    /// Create a new mix
    Add { name: String },
    /// Remove a mix
    Remove { name: String },
    /// Show or select the active mix
    Active { mix: Option<String> },
    /// Set mix volume
    Volume { mix: String, db: f32 },
}

/// Runs a mix command against the daemon.
pub(super) async fn run(action: MixAction) {
    match action {
        MixAction::List => match pipelink_ipc::client::send_query(ControlEvent::ListMixes).await {
            Ok(ControlReply::Mixes { names }) => {
                for name in names {
                    println!("{name}");
                }
            },
            Ok(other) => fail(format!("unexpected answer: {other:?}")),
            Err(e) => fail(e),
        },
        MixAction::Add { name } => {
            let target = MixName::from(name.clone());
            let request = ControlEvent::CreateMix {
                name: target.clone(),
            };
            match pipelink_ipc::client::send_mutation(request, |event| {
                confirms_created(event, &target)
            })
            .await
            {
                Ok(StateEvent::MixOperationFailed { error, .. }) => {
                    fail(format!("failed to create mix '{name}': {error}"));
                },
                Ok(_) => println!("Mix '{name}' created successfully"),
                Err(e) => fail(e),
            }
        },
        MixAction::Remove { name } => {
            let target = MixName::from(name.clone());
            let request = ControlEvent::RemoveMix {
                name: target.clone(),
            };
            match pipelink_ipc::client::send_mutation(request, |event| {
                confirms_removed(event, &target)
            })
            .await
            {
                Ok(StateEvent::MixOperationFailed { error, .. }) => {
                    fail(format!("failed to remove mix '{name}': {error}"));
                },
                Ok(StateEvent::MixRemoved { mix }) => println!("Mix '{mix}' removed"),
                Ok(_) => println!("Mix '{name}' removed"),
                Err(e) => fail(e),
            }
        },
        MixAction::Active { mix: None } => {
            match pipelink_ipc::client::send_query(ControlEvent::GetActiveMix).await {
                Ok(ControlReply::ActiveMix { mix: Some(mix) }) => println!("{mix}"),
                Ok(ControlReply::ActiveMix { mix: None }) => println!("(none)"),
                Ok(other) => fail(format!("unexpected answer: {other:?}")),
                Err(e) => fail(e),
            }
        },
        MixAction::Active { mix: Some(name) } => {
            let target = MixName::from(name.clone());
            let request = ControlEvent::SelectGlobalActiveLayer {
                mix: target.clone(),
            };
            let reply = pipelink_ipc::client::send_mutation(request, |event| {
                confirms_activated(event, &target)
            })
            .await;
            match reply {
                Ok(StateEvent::LayerSelectionFailed { error, .. }) => {
                    fail(format!("failed to select active mix '{name}': {error}"));
                },
                Ok(StateEvent::GlobalLayerSwitched { mix }) => {
                    println!("Active mix set to '{mix}'");
                },
                Ok(other) => fail(format!("unexpected answer: {other:?}")),
                Err(e) => fail(e),
            }
        },
        MixAction::Volume { mix, db } => {
            fail(format!(
                "setting the volume of mix '{mix}' to {db} dB is not implemented yet"
            ));
        },
    }
}

/// Whether the state event confirms that the mix was created.
fn confirms_created(event: &StateEvent, target: &MixName) -> bool {
    matches!(
        event,
        StateEvent::MixAdded { mix } | StateEvent::MixOperationFailed { mix, .. }
            if mix.eq_ignore_ascii_case(target)
    )
}

/// Whether the state event confirms that the mix was removed.
fn confirms_removed(event: &StateEvent, target: &MixName) -> bool {
    matches!(
        event,
        StateEvent::MixRemoved { mix } | StateEvent::MixOperationFailed { mix, .. }
            if mix.eq_ignore_ascii_case(target)
    )
}

/// Whether the state event confirms that the active mix switched or failed.
fn confirms_activated(event: &StateEvent, target: &MixName) -> bool {
    matches!(
        event,
        StateEvent::GlobalLayerSwitched { mix } | StateEvent::LayerSelectionFailed { mix, .. }
            if mix.eq_ignore_ascii_case(target)
    )
}
