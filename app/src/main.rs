/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use clap::{
    Parser, Subcommand,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};

fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Green.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Cyan.on_default())
}

#[derive(Parser)]
#[command(name = "pipelink", author, version, about = "Pipelink audio routing control tool", styles = cli_styles())]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(long, help = "Starts the GUI")]
    gui: bool,

    #[arg(long, help = "Shows daemon status")]
    status: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
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
}

#[derive(Subcommand, Debug, Clone)]
enum RouteAction {
    /// List active routes
    List,
    /// Connect source to destination
    Set { source: String, destination: String },
    /// Remove route
    Remove { source: String, destination: String },
}

#[derive(Subcommand, Debug, Clone)]
enum ChannelAction {
    /// List all configured channels
    List,
    /// Create a new channel
    Add { name: String },
    /// Remove a channel
    Remove { name: String },
}

#[derive(Subcommand, Debug, Clone)]
enum MixAction {
    /// List all configured mixes
    List,
    /// Create a new mix
    Add { name: String },
    /// Set mix volume
    Volume { mix: String, db: f32 },
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    if args.gui {
        #[cfg(feature = "gui")]
        {
            gui::run().await;
        }
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("You need to enable the `gui` feature");
            std::process::exit(1);
        }
    }
    run_cli(args);
}

fn run_cli(args: Cli) {
    if args.status {
        println!("Status");
    }

    match args.command {
        Command::Route { action } => match action {
            RouteAction::List => println!("Listing active audio routes..."),
            RouteAction::Set {
                source,
                destination,
            } => {
                println!("Routing {source} -> {destination}");
            },
            RouteAction::Remove {
                source,
                destination,
            } => {
                println!("Removing route {source} -> {destination}");
            },
        },
        Command::Channel { action } => match action {
            ChannelAction::List => println!("Listing channels..."),
            ChannelAction::Add { name } => println!("Added channel: {name}"),
            ChannelAction::Remove { name } => println!("Removed channel: {name}"),
        },
        Command::Mix { action } => match action {
            MixAction::List => println!("Listing mixes..."),
            MixAction::Add { name } => println!("Added mix: {name}"),
            MixAction::Volume { mix, db } => println!("Mix {mix} set to {db} dB"),
        },
    }
}
