//
// monitor.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Aug 17 2022
//
use clap::{Parser, Subcommand};

use crate::CommandContext;

use canopen_eds::{CobId, ValueType, PdoDecoder, Eds};
use canopen_client::{CanOpenFrame, Pdo, NodeId};


use embedded_hal::can::Frame;
use futures_util::stream::StreamExt;

use tokio_socketcan::{CanFrame, CANSocket};

use std::{
    collections::{BTreeMap, HashMap},
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant}
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Span,
    widgets::{Block, Borders, List, ListItem},
    Frame as UiFrame, Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(short = 'n', long = "node-id")]
    node_id: u8,
    #[clap(short = 'f', long = "eds-file")]
    eds_file: String,
}

struct App {
    /// CAN device name
    device_name: String,
    /// Specified node id
    node_id: NodeId,

    /// PDO decoders
    decoders: HashMap<Pdo, PdoDecoder>,

    /// Tracked values
    objects: BTreeMap<CobId, ValueType>,
}

impl App {
    pub fn new(device_name: String, node_id: u8, eds: Eds) -> App {
        let decoders = [
            (Pdo::Tx1, eds.get_tpdo1_decoder()),
            (Pdo::Tx2, eds.get_tpdo2_decoder()),
            (Pdo::Tx3, eds.get_tpdo3_decoder()),
            (Pdo::Tx4, eds.get_tpdo4_decoder())
        ]
        .into_iter()
        .filter_map(|(k, v)| if let Some(v) = v { Some((k, v)) } else { None })
        .collect::<HashMap<_, _>>();

        App { 
            device_name,
            node_id: node_id.into(),
            decoders,
            objects: Default::default(),
        }
    }

    pub fn update(&mut self, frame: CanFrame) {
        if let Ok(frame) = canopen_client::parse(frame) {
            match frame {
                (Some(node_id), CanOpenFrame::Pdo(pdo_channel, data)) if node_id == self.node_id => {
                    if let Some(decoder) = self.decoders.get(&pdo_channel) {
                        for (cobid, value) in decoder.decode(&data.data[..]).into_iter().filter_map(|v| v) {
                            self.objects.insert(cobid, value);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

pub async fn run(args: Args, ctx: CommandContext) -> anyhow::Result<()> {
    let socket = ctx.socket;
    let device = ctx.device;
    let tick_rate = ctx.tick_rate;

    let node_id = args.node_id;

    let eds = Eds::from(args.eds_file)?;

    let app = Arc::new(Mutex::new(App::new(device, node_id, eds)));
    
    let ui_task = tokio::spawn(ui_task(app.clone(), tick_rate));
    tokio::spawn(frame_processor_task(socket, app));

    tokio::join!(ui_task).0??;

    Ok(())
}

async fn frame_processor_task(mut socket: CANSocket, app: Arc<Mutex<App>>) -> anyhow::Result<()> {
    while let Some(Ok(frame)) = socket.next().await {
        let mut app = app.lock().unwrap();
        app.update(frame);
    }

    Ok(())
}

async fn ui_task(app: Arc<Mutex<App>>, tick_rate: u64) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        // The below is in a new scope block so app is out of scope before `await` is called.
        {
            let mut app = app.lock().unwrap();
            terminal.draw(|f| ui(f, &*app))?;

            if crossterm::event::poll(Duration::from_millis(10))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(tick_rate)).await;
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui<B: Backend>(f: &mut UiFrame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(100),
            ]
            .as_ref(),
        )
        .split(f.size());

    // let format_mode = app.format_mode;

    let items: Vec<ListItem> = app.objects.iter().map(|(cobid, value)| {

        let (index, subindex) = cobid.clone().into_parts();

        let line = Span::from(
                        Span::styled(format!("{:02X}.{:01X} {:?}", index, subindex, value),
                        Style::default())
                    );
        ListItem::new(line)
    }).collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(app.device_name.as_str()));

    f.render_widget(list, chunks[0]);
}
