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
    /// Rename an existing mix
    Rename { old_name: String, new_name: String },
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
        MixAction::Rename { old_name, new_name } => rename_mix(&old_name, &new_name).await,
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

/// Whether the state event confirms or rejects a mix rename.
fn confirms_renamed(event: &StateEvent, target: &MixName) -> bool {
    matches!(
        event,
        StateEvent::MixRenamed { old_name, .. } if old_name.eq_ignore_ascii_case(target)
    ) || matches!(
        event,
        StateEvent::MixOperationFailed { mix, .. } if mix.eq_ignore_ascii_case(target)
    )
}

async fn rename_mix(old_name: &str, new_name: &str) {
    let old_target = MixName::from(old_name.to_string());
    let new_target = MixName::from(new_name.to_string());
    let request = ControlEvent::RenameMix {
        old_name: old_target.clone(),
        new_name: new_target.clone(),
    };
    match pipelink_ipc::client::send_mutation(request, |event| confirms_renamed(event, &old_target))
        .await
    {
        Ok(StateEvent::MixOperationFailed { error, .. }) => {
            fail(format!(
                "failed to rename mix '{old_name}' to '{new_name}': {error}"
            ));
        },
        Ok(StateEvent::MixRenamed { old_name, new_name }) => {
            println!("Mix '{old_name}' renamed to '{new_name}'");
        },
        Ok(_) => println!("Mix '{old_name}' renamed to '{new_name}'"),
        Err(e) => fail(e),
    }
}
