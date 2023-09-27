//
// dump.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//

use crate::CommandContext;
use crate::utils;

use socketcan::{CanFrame, tokio::CanSocket};

use embedded_can::Frame;
use futures_util::stream::StreamExt;

use std::{
    collections::BTreeMap,
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

/// Track information on received CAN frames
struct TrackedFrame {
    // The CAN frame
    pub frame: CanFrame,
    // Time point of when this CAN frame from received
    pub recv_time: Instant,
    // Delta since the last frame
    pub delta: f32,
}

impl TrackedFrame {
    pub fn new(frame: CanFrame, recv_time: Instant, delta: f32) -> Self {
        Self { frame, recv_time, delta }
    }
}

#[derive(Debug, Clone, Copy)]
enum DataFormatMode {
    Hex,
    Binary,
}

struct App {
    pub frames: BTreeMap<u32, TrackedFrame>,
    pub device_name: String,
    pub format_mode: DataFormatMode,
}

impl App {
    pub fn new(device_name: String) -> Self {
        Self {
            frames: BTreeMap::default(),
            device_name,
            format_mode: DataFormatMode::Hex,
        }
    }
}

impl App {
    pub fn update(&mut self, frame: CanFrame) {
        let now = Instant::now();
        let id = utils::id_to_raw(&frame.id());

        // Get delta with the last received frame of this ID
        let delta = self.frames.get(&id).map_or(0.0, |f| (now - f.recv_time).as_secs_f32());

        self.frames.insert(id, TrackedFrame::new(frame, now, delta));
    }

    pub fn cycle_display_format(&mut self) {
        // Step through format modes
        // TODO: Use an iterator here?
        self.format_mode = match self.format_mode {
            DataFormatMode::Hex => DataFormatMode::Binary,
            DataFormatMode::Binary => DataFormatMode::Hex,
        };
    }
}

pub async fn run(ctx: CommandContext) -> anyhow::Result<()> {
    let socket = ctx.socket;
    let device = ctx.interface;
    let tick_rate = ctx.tick_rate;

    // let tick_rate = Duration::from_millis(250);

    let app = Arc::new(Mutex::new(App::new(device)));

    let ui_task = tokio::spawn(ui_task(app.clone(), tick_rate));
    tokio::spawn(frame_processor_task(socket, app));

    // TODO: Use the nested results...
    tokio::join!(ui_task).0??;

    Ok(())
}

async fn frame_processor_task(mut socket: CanSocket, app: Arc<Mutex<App>>) -> anyhow::Result<()> {
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
                        KeyCode::Char('t') => app.cycle_display_format(),
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

    let format_mode = app.format_mode;

    let items: Vec<ListItem> = app.frames.iter().map(|(_, frame)| {
        let TrackedFrame { frame, recv_time: _, delta } = frame;

        let id = utils::id_to_raw(&frame.id());
        let dlc = frame.dlc();
        let data_string = frame.data().iter().fold(String::from(""), |a, b| {
            match format_mode {
                DataFormatMode::Hex => format!("{} {:02X}", a, b),
                DataFormatMode::Binary => format!("{} {:08b}", a, b),
            }
        });

        let line = Span::from(
                        Span::styled(format!("{:.3} {:08X} [{}] {}", delta, id, dlc, data_string),
                        Style::default())
                    );
        ListItem::new(line)
    }).collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(app.device_name.as_str()));

    f.render_widget(list, chunks[0]);
}
