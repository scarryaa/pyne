use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use cursor_movement::CursorMovement;
use editor::Editor;
use gutter::Gutter;
use mode::Mode;
use ratatui::{
    crossterm::{self, event::KeyModifiers},
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Terminal,
};
use std::{error::Error, io, str::FromStr};

mod cursor_movement;
mod editor;
mod gutter;
mod mode;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut editor = Editor::new();

    let res = run_app(&mut terminal, &mut editor);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    editor: &mut Editor,
) -> Result<(), Box<dyn Error>> {
    loop {
        let (cursor_line, cursor_column) = editor.get_cursor_screen_position();
        let (scroll_x, scroll_y) = editor.get_scroll_offset();

        terminal.draw(|f| {
            let area = f.area();

            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Min(1),    // Main editor area
                        Constraint::Length(1), // Status line
                        Constraint::Length(1), // Debug info
                    ]
                    .as_ref(),
                )
                .split(area);

            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Length(6), // Gutter
                        Constraint::Min(1),    // Main editor area
                    ]
                    .as_ref(),
                )
                .split(vertical_chunks[0]);

            editor.set_viewport((
                horizontal_chunks[1].width as usize,
                vertical_chunks[0].height as usize,
            ));

            let line_numbers = Gutter::get_visible_line_numbers(editor);
            let gutter_content = Paragraph::new(line_numbers.join("\n"))
                .style(Style::default().fg(Color::from_u32(0x404040)));
            f.render_widget(gutter_content, horizontal_chunks[0]);

            let content = Paragraph::new(editor.get_visible_content()).block(Block::default());
            f.render_widget(content, horizontal_chunks[1]);

            let cursor_screen_x = (cursor_column as i32 - scroll_x as i32).max(0) as u16;
            let cursor_screen_y = (cursor_line as i32 - scroll_y as i32).max(0) as u16;

            f.set_cursor(
                horizontal_chunks[1].x + cursor_screen_x,
                vertical_chunks[0].y + cursor_screen_y,
            );

            // Render status line
            let mode_text = format!(" {} ", editor.get_mode());
            let line_col_text = format!("{}:{} ", cursor_line + 1, cursor_column + 1);
            let status_line = Paragraph::new(mode_text + &line_col_text)
                .style(Style::default().bg(Color::from_u32(0x202020)));
            f.render_widget(status_line, vertical_chunks[1]);

            // Render debug info if enabled
            if editor.is_debug_info_visible() {
                let debug_info = Paragraph::new(editor.get_debug_info());
                f.render_widget(debug_info, vertical_chunks[2]);
            }
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match editor.get_mode() {
                    Mode::Normal => match (key.modifiers, key.code) {
                        (KeyModifiers::NONE, KeyCode::Char('q')) => break,
                        (KeyModifiers::NONE, KeyCode::Char('i')) => editor.set_mode(Mode::Insert),
                        (KeyModifiers::SHIFT, KeyCode::Char('D')) => editor.toggle_debug_info(),
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
                        KeyCode::Tab => editor.insert_str("    ".to_string()),
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
    Ok(())
}
