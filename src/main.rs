use std::{
    error::Error,
    io::{self},
};

use cursor_movement::CursorMovement;
use editor::Editor;
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::Position,
    prelude::CrosstermBackend,
    widgets::{Block, Paragraph},
    Terminal,
};

mod cursor_movement;
mod editor;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut editor = Editor::new();
    terminal.show_cursor()?;

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let content = Paragraph::new(editor.to_string()).block(Block::default());
            f.render_widget(content, area);
        })?;

        terminal.show_cursor()?;
        let (line, column) = editor.get_cursor_screen_position();
        terminal.set_cursor_position(Position::new(column as u16, line as u16))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char(c) => editor.insert(c),
                KeyCode::Backspace => editor.delete(),
                KeyCode::Enter => editor.insert_new_line(),
                KeyCode::Left => editor.move_cursor(CursorMovement::LEFT),
                KeyCode::Right => editor.move_cursor(CursorMovement::RIGHT),
                KeyCode::Up => editor.move_cursor(CursorMovement::UP),
                KeyCode::Down => editor.move_cursor(CursorMovement::DOWN),
                KeyCode::Home => todo!(),
                KeyCode::End => todo!(),
                KeyCode::PageUp => todo!(),
                KeyCode::PageDown => todo!(),
                KeyCode::Tab => todo!(),
                KeyCode::BackTab => todo!(),
                KeyCode::Delete => todo!(),
                KeyCode::Insert => todo!(),
                KeyCode::F(_) => todo!(),
                KeyCode::Null => todo!(),
                KeyCode::Esc => todo!(),
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
