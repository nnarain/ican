//
// send.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jan 16 2023
//

use crate::CommandContext;
use clap::Parser;

use socketcan::{tokio::CanSocket, CanFrame};
use embedded_can::{Frame, StandardId};
use thiserror::Error;
use std::time::Duration;

#[derive(Error, Debug)]
pub enum SendError {
    #[error("Failed to parse frame from input")]
    SyntaxError
}

#[derive(Parser, Debug)]
pub struct Args {
    frame: String,
    #[clap(short = 'r', long = "rate", value_parser)]
    rate: Option<f32>,
}

pub async fn run(ctx: CommandContext, args: Args) -> anyhow::Result<()> {
    let frame = build_frame(&args.frame)?;

    let period = args.rate
        .filter(|&r| r != 0.0)
        .map(|r| Duration::from_secs_f32(1.0 / r));

    tokio::spawn(send_task(ctx.socket, frame, period));
    tokio::signal::ctrl_c().await?;

    Ok(())
}

async fn send_task(socket: CanSocket, frame: CanFrame, dur: Option<Duration>) -> anyhow::Result<()> {

    loop {
        socket.write_frame(frame.clone())?.await?;

        match dur {
            None => break,
            Some(dur) => tokio::time::sleep(dur).await,
        }
    }

    Ok(())
}

fn build_frame(text: &str) -> Result<CanFrame, SendError> {
    let parts: Vec<_> = text.split('#').collect();
    if parts.len() == 2 {
        let id = u16::from_str_radix(parts[0], 16).map_err(|_| SendError::SyntaxError)?;

        let body = parts[1];
        if body.len() % 2 == 0 {
            let data = body
                .chars()
                .collect::<Vec<char>>()
                .chunks(2)
                .map(|s| u8::from_str_radix(String::from_iter(s).as_str(), 16).map_err(|_| SendError::SyntaxError))
                .collect::<Result<Vec<u8>, SendError>>()?;

            StandardId::new(id)
                .map(|id| CanFrame::new(id, &data[..]))
                .flatten()
                .ok_or(SendError::SyntaxError)
        }
        else {
            Err(SendError::SyntaxError)
        }
    }
    else {
        Err(SendError::SyntaxError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_standard_frame1() {
        let text = "705#05";

        let frame = build_frame(text).unwrap();
        assert_eq!(frame.id(), embedded_can::Id::Standard(StandardId::new(0x705).unwrap()));
        assert_eq!(frame.dlc(), 1);
        assert_eq!(frame.data(), &[0x05]);
    }

    #[test]
    fn build_standard_frame2() {
        let text = "705#0102";

        let frame = build_frame(text).unwrap();
        assert_eq!(frame.id(), embedded_can::Id::Standard(StandardId::new(0x705).unwrap()));
        assert_eq!(frame.dlc(), 2);
        assert_eq!(frame.data(), &[0x01, 0x02]);
    }
}
