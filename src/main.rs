//
// main.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//
use clap::Parser;

use ican::{
    drivers::AsyncCanDriverPtr,
    CommandContext, Args, Command, action,
};

use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let args = Args::parse();

    let interface = args.interface.clone().to_string();
    let tick_rate = args.tui_tick_rate;

    let driver: AsyncCanDriverPtr = args.interface.try_into()?;

    let context = CommandContext { driver, interface, tick_rate};

    match args.cmd {
        Command::Dump => Ok(action::dump::run(context).await?),
        Command::Monitor => Ok(action::monitor::run(context).await?),
        Command::Send(args) => Ok(action::send::run(context, args).await?),
        Command::Bridge(args) => Ok(action::bridge::run(context, args).await?),
    }
}

