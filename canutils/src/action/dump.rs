//
// dump.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 31 2022
//
use crate::{CommandContext, utils};
use socketcan::tokio::CanSocket;
use futures_util::stream::StreamExt;
use embedded_can::Frame;

pub async fn run(ctx: CommandContext) -> anyhow::Result<()> {
    tokio::spawn(dump_task(ctx.socket));

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn dump_task(mut socket: CanSocket) -> anyhow::Result<()> {
    while let Some(Ok(frame)) = socket.next().await {
        let id = utils::id_to_raw(&frame.id());
        let dlc = frame.dlc();
        let data_string = frame.data().iter().fold(String::from(""), |a, b| format!("{} {:02X}", a, b));
        println!("{:08X} [{}] {}", id, dlc, data_string);
    }

    Ok(())
}
