//
// main.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 04 2022
//

use anyhow::Context;
use clap::Parser;

use socketcan::CanSocket;
use embedded_hal::can::{Can, Frame, Id};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;


#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// CAN interface
    #[clap(value_parser)]
    interface: String,
}


fn main() -> anyhow::Result<()> {

    let args = Args::parse();
    let can_interface = args.interface;

    let mut socket = CanSocket::open(&can_interface)
        .with_context(|| format!("Failed to open socket on interface {}", can_interface))?;
    socket.set_nonblocking(true).with_context(|| format!("Failed to make socket non-blocking"))?;

    let shutdown = AtomicBool::new(false);
    let shutdown = Arc::new(shutdown);
    let signal_shutdown = shutdown.clone();

    ctrlc::set_handler(move ||{
        signal_shutdown.store(true, Ordering::Relaxed);
    })
    .expect("Failed to set signal handler");

    while !shutdown.load(Ordering::Relaxed) {
        match socket.receive() {
            Ok(frame) => println!("{}", frame_to_string(&frame)),
            Err(_) => {},
        }
    }

    Ok(())
}

fn frame_to_string<F: Frame>(f: &F) -> String {
    let id = {
        match f.id() {
            Id::Standard(id) => id.as_raw() as u32,
            Id::Extended(id) => id.as_raw(),
        }
    };

    let data_string = f.data().iter().fold(String::from(""), |a, b| format!("{} {:02x}", a, b));

    format!("{:08X}  [{}] {}", id, f.dlc(), data_string)
}
