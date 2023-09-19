//
// lib.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//
pub mod action;
pub mod utils;

use socketcan::tokio::CanSocket;

use clap::{Parser, Subcommand};

/// canutils provides several common CAN commands
#[derive(Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: Command,
    #[clap(short = 'd', long = "device", value_parser)]
    pub device: String,
    #[clap(short = 't', long = "tick-rate", default_value = "200")]
    pub tick_rate: u64,
}

pub enum Driver {
    SocketCan,
}

/// Command to run
#[derive(Subcommand)]
pub enum Command {
    /// Print CAN frames to console
    Dump,
    /// TUI displaying CAN frames and decoded signals
    Monitor,
    /// Send CAN frames to the selected interface
    Send(action::send::Args),
    /// Bridge different CAN interfaces together
    Bridge,
    /// CANopen subcommands
    #[clap(subcommand)]
    Canopen(action::canopen::CanOpenCommands),
}

/// Subcommand context
pub struct CommandContext {
    pub socket: CanSocket,
    pub device: String,
    pub tick_rate: u64,
}
