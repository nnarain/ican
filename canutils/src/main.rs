use anyhow::Context;
//
// main.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//
use tokio;
use tokio_socketcan::CANSocket;
use clap::Parser;

use canutils::{CommandContext, Args, Command, action};


#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let args = Args::parse();
    let device = args.device;
    let tick_rate = args.tick_rate;

    let socket = CANSocket::open(&device)
                                .with_context(|| format!("Failed to open CAN interface {}", device))?;

    let ctx = CommandContext {socket, device, tick_rate};

    match args.cmd {
        Command::Dump => Ok(action::dump::run(ctx).await?),
        Command::Monitor => Ok(action::monitor::run(ctx).await?),
        Command::Send(args) => Ok(action::send::run(ctx, args).await?),
        Command::Bridge => Ok(()),
        Command::Canopen(cmd) => Ok(action::canopen::run(cmd, ctx).await?)
    }
}
