//
// dump.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//

use crate::CommandContext;
use crate::utils;

use tokio_socketcan::{CanFrame, CANSocket};

use embedded_hal::can::Frame;
use futures_util::stream::StreamExt;

use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
    time::Duration
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

#[derive(Default)]
struct App {
    pub frames: BTreeMap<u32, CanFrame>,
}

impl App {
    pub fn update(&mut self, frame: CanFrame) {
        let id = utils::id_to_raw(&frame.id());
        self.frames.insert(id, frame);
    }
}

pub async fn run(ctx: CommandContext) -> anyhow::Result<()> {
    let socket = ctx.socket;

    // let tick_rate = Duration::from_millis(250);

    let app = Arc::new(Mutex::new(App::default()));

    let ui_task = tokio::spawn(ui_task(app.clone()));
    tokio::spawn(frame_processor_task(socket, app));

    // TODO: Use the nested results...
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

async fn ui_task(app: Arc<Mutex<App>>) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        {
            let app = app.lock().unwrap();
            terminal.draw(|f| ui(f, &*app))?;
        }

        if crossterm::event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
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

    let items: Vec<ListItem> = app.frames.iter().map(|(_, frame)| {
        let id = utils::id_to_raw(&frame.id());
        let dlc = frame.dlc();
        let data_string = frame.data().iter().fold(String::from(""), |a, b| format!("{} {:02X}", a, b));

        let line = Span::from(
                        Span::styled(format!("{:08X} [{}] {}", id, dlc, data_string),
                        Style::default())
                    );
        ListItem::new(line)
    }).collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("vcan0"));

    f.render_widget(list, chunks[0]);
}
