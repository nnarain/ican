use anyhow::Context;
//
// main.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//

use socketcan::tokio::CanSocket;
use clap::Parser;

use ican::{CommandContext, Args, Command, action, DriverOpts};

use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let args = Args::parse();

    let interface = args.interface;
    let tick_rate = args.tui_tick_rate;

    // TODO(nnarain): Support other drivers
    if let DriverOpts::SocketCan(interface) = interface {
        let socket = CanSocket::open(&interface)
            .with_context(|| format!("Failed to open CAN interface {}", interface))?;

        let ctx = CommandContext {socket, interface, tick_rate};

        match args.cmd {
            Command::Dump => Ok(action::dump::run(ctx).await?),
            Command::Monitor => Ok(action::monitor::run(ctx).await?),
            Command::Send(args) => Ok(action::send::run(ctx, args).await?),
            Command::Bridge => Ok(()),
            // Command::Canopen(cmd) => Ok(action::canopen::run(cmd, ctx).await?)
        }
    }
    else {
        Ok(())
    }

}
