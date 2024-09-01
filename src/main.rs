use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use cursor_movement::CursorMovement;
use editor::Editor;
use mode::Mode;
use ratatui::{
    crossterm,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    text::Line,
    widgets::{Block, Paragraph},
    Terminal,
};
use std::{error::Error, io};

mod cursor_movement;
mod editor;
mod mode;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut editor = Editor::new();

    loop {
        let (cursor_line, cursor_column) = editor.get_cursor_screen_position();

        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(area);

            let content = Paragraph::new(editor.to_string()).block(Block::default());
            f.render_widget(content, chunks[0]);
            f.set_cursor(cursor_column as u16, cursor_line as u16);

            let status_line = Line::raw(format!(" {} ", editor.get_mode()));
            f.render_widget(status_line, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match editor.get_mode() {
                    Mode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('i') => editor.set_mode(Mode::Insert),
                        _ => {}
                    },
                    Mode::Insert => match key.code {
                        KeyCode::Char(c) => editor.insert(c),
                        KeyCode::Backspace => editor.delete(),
                        KeyCode::Enter => editor.insert_new_line(),
                        KeyCode::Left => editor.move_cursor(CursorMovement::Left),
                        KeyCode::Right => editor.move_cursor(CursorMovement::Right),
                        KeyCode::Up => editor.move_cursor(CursorMovement::Up),
                        KeyCode::Down => editor.move_cursor(CursorMovement::Down),
                        KeyCode::Home => editor.move_cursor(CursorMovement::LineStart),
                        KeyCode::End => editor.move_cursor(CursorMovement::LineEnd),
                        KeyCode::PageUp => todo!(),
                        KeyCode::PageDown => todo!(),
                        KeyCode::Tab => todo!(),
                        KeyCode::BackTab => todo!(),
                        KeyCode::Delete => todo!(),
                        KeyCode::Insert => todo!(),
                        KeyCode::F(_) => todo!(),
                        KeyCode::Null => todo!(),
                        KeyCode::Esc => editor.set_mode(Mode::Normal),
                        KeyCode::CapsLock => todo!(),
                        KeyCode::ScrollLock => todo!(),
                        KeyCode::NumLock => todo!(),
                        KeyCode::PrintScreen => todo!(),
                        KeyCode::Pause => todo!(),
                        KeyCode::Menu => todo!(),
                        KeyCode::KeypadBegin => todo!(),
                        KeyCode::Media(_) => todo!(),
                        KeyCode::Modifier(_) => todo!(),
                        _ => {}
                    },
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}
