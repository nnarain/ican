//
// main.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 04 2022
//

use clap::Parser;

use socketcan::CanSocket;

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

    let socket = CanSocket::open(&args.interface)?;
    socket.set_nonblocking(true).expect("Failed to make socket non-blocking");

    let shutdown = AtomicBool::new(false);
    let shutdown = Arc::new(shutdown);
    let signal_shutdown = shutdown.clone();

    ctrlc::set_handler(move ||{
        signal_shutdown.store(true, Ordering::Relaxed);
    })
    .expect("Failed to set signal handler");

    while !shutdown.load(Ordering::Relaxed) {
        match socket.read_frame() {
            Ok(frame) => println!("{:?}", frame),
            Err(_) => {},
        }
    }

    Ok(())
}
