use std::{
    fs::{self, read_to_string},
    io::{self, stdout},
    path::PathBuf,
    process::Command,
    thread,
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fork::{fork, Fork};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct GlobalInfo {
    config_path: Option<PathBuf>,
    list: Vec<Program>,
    liststate: ListState,
    list_pos: usize,
}

#[derive(Serialize, Deserialize)]
struct Program {
    title: String,
    description: String,
    command: String,
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut data = handle_setup();

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| ui(f, &mut data))?;
        should_quit = handle_events(&mut data).unwrap();
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_events(data: &mut GlobalInfo) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            } else if key.code == KeyCode::Up || key.code == KeyCode::Char('k') && data.list_pos > 0
            {
                data.list_pos -= 1;
            } else if key.code == KeyCode::Down
                || key.code == KeyCode::Char('j') && data.list_pos < data.list.len() - 1
            {
                data.list_pos += 1;
            } else if key.code == KeyCode::Enter {
                let command = data.list[data.list_pos].command.clone();
                match fork() {
                    Ok(Fork::Parent(child)) => {
                        println!(
                            "Continuing execution in parent process, new child has pid: {}",
                            child
                        );
                    }
                    Ok(Fork::Child) => {
                        Command::new("sh").arg("-c").arg(command).spawn().unwrap();
                    }
                    Err(_) => println!("Fork failed"),
                }
                thread::sleep(Duration::from_millis(5000));
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn ui(frame: &mut Frame, data: &mut GlobalInfo) {
    let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ],
    )
    .split(frame.size());
    frame.render_widget(
        Block::new().borders(Borders::TOP).title("GLauncher"),
        main_layout[0],
    );

    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(main_layout[1]);
    let mut items = Vec::new();
    for item in &data.list {
        items.push(item.title.clone())
    }

    let list = List::new(items)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .highlight_symbol(">>");

    data.liststate.select(Some(data.list_pos));
    frame.render_stateful_widget(list, inner_layout[0], &mut data.liststate);

    let right_layout = Layout::new(
        Direction::Vertical,
        [Constraint::Length(3), Constraint::Min(0)],
    )
    .split(inner_layout[1]);
    frame.render_widget(
        Paragraph::new(data.list[data.list_pos].title.clone()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        right_layout[0],
    );
    frame.render_widget(
        Paragraph::new(data.list[data.list_pos].description.clone()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        right_layout[1],
    );
    frame.render_widget(
        Paragraph::new(data.list[data.list_pos].command.clone()).block(
            Block::default()
                .title("Command")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        main_layout[2],
    )
}

fn handle_setup() -> GlobalInfo {
    let mut data = GlobalInfo::default();
    let config_path = dirs::config_dir().unwrap().join("glauncher");

    if !config_path.exists() {
        if let Err(e) = fs::create_dir_all(config_path.clone()) {
            eprintln!("Could not make config directory. {}", e)
        };
    }
    data.config_path = Some(config_path);
    for file in fs::read_dir(data.config_path.as_ref().unwrap()).unwrap() {
        let contents = read_to_string(file.unwrap().path());
        match toml::from_str::<Program>(contents.unwrap().as_str()) {
            Ok(entry) => data.list.push(entry),
            Err(_) => {
                eprintln!("Could not load config file as it is invalid.")
            }
        }
    }
    data
}
