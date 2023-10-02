//
// bridge.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Oct 01 2023
//

use crate::{DriverOpts, CommandContext, drivers::AsyncCanDriverPtr};

use clap::Parser;

/// Arguments for the bridge command
#[derive(Debug, Parser)]
pub struct Args {
    /// The CAN interface to bridge to
    #[arg(value_enum, value_parser = clap::value_parser!(DriverOpts))]
    pub interface: DriverOpts,
}

pub async fn run(ctx: CommandContext, args: Args) -> anyhow::Result<()> {
    let from_driver: AsyncCanDriverPtr = ctx.driver;
    let to_driver: AsyncCanDriverPtr = args.interface.try_into()?;

    tokio::spawn(bridge_task(from_driver, to_driver));

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn bridge_task(mut from_driver: AsyncCanDriverPtr, mut to_driver: AsyncCanDriverPtr) -> anyhow::Result<()> {
    while let Some(frame) = from_driver.recv().await {
        to_driver.send(frame).await;
    }

    Ok(())
}
