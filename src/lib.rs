//
// lib.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//
pub mod action;
pub mod utils;

use std::str::FromStr;
use regex::Regex;

use socketcan::tokio::CanSocket;
use clap::{Parser, Subcommand};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IcanParseErrors {
    #[error("Invalid device options")]
    InvalidDriver,
}

/// CAN driver options
#[derive(Debug, Clone, PartialEq)]
pub enum DriverOpts {
    /// SocketCAN driver. Options: interface
    SocketCan(String),
    /// UDP tunneling. Options: IP, port
    Udp(String, u16),
}

impl FromStr for DriverOpts {
    type Err = IcanParseErrors;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"([\w]+):\/\/([\w]+)").unwrap();

        // Attempt to match the specified driver
        // This takes the form:
        //   - socketcan://interface
        //   - udp://127.0.0.1:8000
        if let Some(caps) = re.captures(s) {
            let (_, [driver, opts]) = caps.extract();

            match driver {
                "socketcan" => {
                    Ok(DriverOpts::SocketCan(opts.to_string()))
                },
                _ => {
                    Err(IcanParseErrors::InvalidDriver)
                }
            }
        }
        else {
            // If the expression doesn't match, just return the string as a SocketCAN driver option
            Ok(DriverOpts::SocketCan(s.to_string()))
        }

    }
}

/// ican provides several common CAN commands
#[derive(Parser, Debug)]
#[command(author = "Natesh Narain", version, about = "Modern CAN tools")]
pub struct Args {
    /// The CAN interface to use (with driver options if applicable)
    #[arg(value_enum, value_parser = clap::value_parser!(DriverOpts))]
    pub interface: DriverOpts,
    #[command(subcommand)]
    pub cmd: Command,
    #[arg(short = 't', long = "tick-rate", default_value = "200")]
    pub tui_tick_rate: u64,
}

/// Command to run
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print CAN frames to console
    Dump,
    /// TUI displaying CAN frames and decoded signals
    Monitor,
    /// Send CAN frames to the selected interface
    Send(action::send::Args),
    /// Bridge different CAN interfaces together
    Bridge,
    // /// CANopen subcommands
    // #[clap(subcommand)]
    // Canopen(action::canopen::CanOpenCommands),
}

/// Subcommand context
pub struct CommandContext {
    pub socket: CanSocket,
    pub interface: String,
    pub tick_rate: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socketcan_driver_opt() {
        let opts = DriverOpts::from_str("socketcan://vcan0").unwrap();
        assert_eq!(opts, DriverOpts::SocketCan("vcan0".to_owned()))
    }

    #[test]
    fn socketcan_driver_opt2() {
        let opts = DriverOpts::from_str("vcan0").unwrap();
        assert_eq!(opts, DriverOpts::SocketCan("vcan0".to_owned()))
    }
}
