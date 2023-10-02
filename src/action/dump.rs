//
// dump.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 31 2022
//
use crate::{drivers::AsyncCanDriverPtr, utils, CommandContext};
use embedded_can::Frame;

pub async fn run(ctx: CommandContext) -> anyhow::Result<()> {
    tokio::spawn(dump_task(ctx.driver));

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn dump_task(mut driver: AsyncCanDriverPtr) -> anyhow::Result<()> {
    while let Some(frame) = driver.recv().await {
        let id = utils::id_to_raw(&frame.id());
        let dlc = frame.dlc();
        let data_string = frame
            .data()
            .iter()
            .fold(String::from(""), |a, b| format!("{} {:02X}", a, b));
        println!("{:08X} [{}] {}", id, dlc, data_string);
    }

    Ok(())
}
