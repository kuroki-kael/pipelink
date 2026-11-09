/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The route commands of the cli.

use clap::Subcommand;

use super::fail;

/// The routes to manage.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum RouteAction {
    /// List active routes
    List,
    /// Connect source to destination
    Set { source: String, destination: String },
    /// Remove route
    Remove { source: String, destination: String },
}

/// Runs a route command, none of which is wired up yet.
pub(super) fn run(action: RouteAction) {
    match action {
        RouteAction::List => fail("listing the routes is not implemented yet"),
        RouteAction::Set {
            source,
            destination,
        } => {
            fail(format!(
                "routing {source} -> {destination} is not implemented yet"
            ));
        },
        RouteAction::Remove {
            source,
            destination,
        } => {
            fail(format!(
                "removing the route {source} -> {destination} is not implemented yet"
            ));
        },
    }
}
