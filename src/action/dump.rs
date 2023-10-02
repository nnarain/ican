//
// dump.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 31 2022
//
use crate::{
    drivers::AsyncCanDriverPtr,
    format::{CanFrameFormatter, DataFormatMode},
    CommandContext,
};

pub async fn run(ctx: CommandContext) -> anyhow::Result<()> {
    tokio::spawn(dump_task(ctx.driver));

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn dump_task(mut driver: AsyncCanDriverPtr) -> anyhow::Result<()> {
    while let Some(frame) = driver.recv().await {
        let fmt: CanFrameFormatter = (frame, DataFormatMode::Hex).into();
        println!("{}", fmt);
    }

    Ok(())
}
