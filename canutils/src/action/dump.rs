//
// dump.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 31 2022
//
use crate::{CommandContext, utils};
use embedded_hal::can::Frame;
use tokio_socketcan::CANSocket;
use futures_util::stream::StreamExt;

pub async fn run(ctx: CommandContext) -> anyhow::Result<()> {
    tokio::spawn(dump_task(ctx.socket));

    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn dump_task(mut socket: CANSocket) -> anyhow::Result<()> {
    while let Some(Ok(frame)) = socket.next().await {
        let id = utils::id_to_raw(&frame.id());
        let dlc = frame.dlc();
        let data_string = frame.data().iter().fold(String::from(""), |a, b| format!("{} {:02X}", a, b));
        println!("{:08X} [{}] {}", id, dlc, data_string);
    }

    Ok(())
}
