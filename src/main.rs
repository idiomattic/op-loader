mod app;
mod event;
mod ui;

use ratatui::DefaultTerminal;
use std::io;

use app::App;

fn run_app(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();

    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &mut app))?;
        event::handle_events(&mut app)?;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| run_app(terminal))
}
