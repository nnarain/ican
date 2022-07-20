//
// dump.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//

use crate::CommandContext;
use crate::utils;

use embedded_hal::can::Frame;
use futures_util::stream::StreamExt;

pub async fn run(ctx: CommandContext) -> anyhow::Result<()> {
    let mut socket = ctx.socket;

    while let Some(Ok(frame)) = socket.next().await {
        let id = utils::id_to_raw(&frame);
        let data = frame.data().iter().fold(String::new(), |a, b| format!("{} {}", a, b));
        println!("{:08X} [{}] {}", id, frame.dlc(), data);
    }

    Ok(())
}
