/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The command line interface of the daemon and the cli.

mod app;
mod channel;
mod mix;
mod output;
mod route;

use std::io;

use app::AppAction;
use channel::ChannelAction;
use clap::{
    CommandFactory, Parser, Subcommand,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};
use mix::MixAction;
use output::OutputAction;
use pipelink_common::events::{ControlEvent, ControlReply};
use route::RouteAction;

/// The parsed command line.
#[derive(Parser)]
#[command(name = "pipelink", author, version, about = "Pipelink audio routing control tool", styles = cli_styles())]
pub(crate) struct Cli {
    /// The command to run.
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long, help = "Starts the GUI")]
    gui: bool,

    #[arg(long, help = "Shows daemon status")]
    status: bool,
}

/// The command to run.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum Command {
    /// Runs the audio routing daemon
    Daemon,
    /// Manage applications and app-to-channel routes
    App {
        #[command(subcommand)]
        action: AppAction,
    },
    /// Manage audio routes
    Route {
        #[command(subcommand)]
        action: RouteAction,
    },
    /// Manage audio channels
    Channel {
        #[command(subcommand)]
        action: ChannelAction,
    },
    /// Manage audio mixes
    Mix {
        #[command(subcommand)]
        action: MixAction,
    },
    /// Manage audio output sinks
    Output {
        #[command(subcommand)]
        action: OutputAction,
    },
}

/// Runs the cli.
pub(crate) async fn run() {
    let args = Cli::parse();

    if args.gui && args.status {
        fail("--gui and --status cannot be combined");
    }

    match args.command {
        Some(command) => {
            if args.gui || args.status {
                fail("--gui and --status cannot be combined with a command");
            }
            run_command(command).await;
        },
        None if args.gui => {
            #[cfg(feature = "gui")]
            {
                crate::gui::run().await;
            }
            #[cfg(not(feature = "gui"))]
            {
                eprintln!("You need to enable the `gui` feature");
                std::process::exit(1);
            }
        },
        None if args.status => print_status().await,
        None => {
            let _ = Cli::command().print_help();
            std::process::exit(2);
        },
    }
}

/// Shows what the daemon reports about the graph.
async fn print_status() {
    let channels = match crate::ipc::send_query(ControlEvent::ListChannels).await {
        Ok(ControlReply::Channels { names }) => names,
        Ok(other) => fail(format!("unexpected answer: {other:?}")),
        Err(e) => fail(daemon_error(&e)),
    };

    let mixes = match crate::ipc::send_query(ControlEvent::ListMixes).await {
        Ok(ControlReply::Mixes { names }) => names,
        Ok(other) => fail(format!("unexpected answer: {other:?}")),
        Err(e) => fail(daemon_error(&e)),
    };

    let app_routes = match crate::ipc::send_query(ControlEvent::ListAppChannels).await {
        Ok(ControlReply::AppChannels { routes }) => routes,
        Ok(other) => fail(format!("unexpected answer: {other:?}")),
        Err(e) => fail(daemon_error(&e)),
    };

    let active_mix = match crate::ipc::send_query(ControlEvent::GetActiveMix).await {
        Ok(ControlReply::ActiveMix { mix }) => mix,
        Ok(other) => fail(format!("unexpected answer: {other:?}")),
        Err(e) => fail(daemon_error(&e)),
    };

    let outputs = match crate::ipc::send_query(ControlEvent::ListOutputs).await {
        Ok(ControlReply::Outputs { outputs }) => outputs,
        Ok(other) => fail(format!("unexpected answer: {other:?}")),
        Err(e) => fail(daemon_error(&e)),
    };

    println!(
        "daemon is running: {} channel(s), {} mix(es), {} app route(s)",
        channels.len(),
        mixes.len(),
        app_routes.len()
    );
    match active_mix {
        Some(mix) => println!("  active mix: {mix}"),
        None => println!("  active mix: (none)"),
    }
    match outputs.iter().find(|o| o.active) {
        Some(out) => println!("  output: {}", output::format_output_display(out)),
        None => println!("  output: (none)"),
    }
    for name in channels {
        println!("  channel: {name}");
    }
    for name in mixes {
        println!("  mix: {name}");
    }
    for (app, channel) in app_routes {
        println!("  app route: {app} -> {channel}");
    }
}

/// Turns a connection error into a message about the daemon.
fn daemon_error(e: &io::Error) -> String {
    match e.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => {
            "the daemon is not running".to_string()
        },
        _ => e.to_string(),
    }
}

/// Runs the command the user asked for.
async fn run_command(command: Command) {
    match command {
        Command::Daemon => {
            if let Err(e) = crate::daemon::run().await {
                eprintln!("Daemon beendet mit Fehler: {e}");
                std::process::exit(1);
            }
        },
        Command::App { action } => app::run(action).await,
        Command::Route { action } => route::run(action).await,
        Command::Channel { action } => channel::run(action).await,
        Command::Mix { action } => mix::run(action).await,
        Command::Output { action } => output::run(action).await,
    }
}

/// Reports the problem and ends the program with a failure code.
pub(crate) fn fail(e: impl std::fmt::Display) -> ! {
    eprintln!("Error: {e}");
    std::process::exit(1);
}

/// The colors of the help output.
fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Green.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Cyan.on_default())
}
