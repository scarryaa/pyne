use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    crossterm,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph},
    Terminal,
};
use std::{error::Error, io};

mod cursor_movement;
mod editor;
mod gutter;
mod mode;

use cursor_movement::CursorMovement;
use editor::Editor;
use gutter::Gutter;
use mode::Mode;

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;
    let mut editor = Editor::new();

    let result = run_app(&mut terminal, &mut editor);

    restore_terminal(&mut terminal)?;
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(|e| e.into())
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    editor: &mut Editor,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| render_ui(f, editor))?;

        if let Event::Key(key) = event::read()? {
            if handle_input(editor, key)? {
                break;
            }
        }
    }
    Ok(())
}

fn render_ui(f: &mut ratatui::Frame, editor: &mut Editor) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(6), Constraint::Min(1)])
        .split(chunks[0]);

    editor.set_viewport((editor_chunks[1].width as usize, chunks[0].height as usize));

    render_gutter(f, editor, editor_chunks[0]);
    render_content(f, editor, editor_chunks[1]);
    render_status_line(f, editor, chunks[1]);
    render_debug_info(f, editor, chunks[2]);

    let (cursor_line, cursor_column) = editor.get_cursor_screen_position();
    let (scroll_x, scroll_y) = editor.get_scroll_offset();
    let cursor_screen_x = (cursor_column as i32 - scroll_x as i32).max(0) as u16;
    let cursor_screen_y = (cursor_line as i32 - scroll_y as i32).max(0) as u16;
    f.set_cursor(
        editor_chunks[1].x + cursor_screen_x,
        chunks[0].y + cursor_screen_y,
    );
}

fn render_gutter(f: &mut ratatui::Frame, editor: &Editor, area: ratatui::layout::Rect) {
    let line_numbers = Gutter::get_visible_line_numbers(editor);
    let gutter_content =
        Paragraph::new(line_numbers.join("\n")).style(Style::default().fg(Color::DarkGray));
    f.render_widget(gutter_content, area);
}

fn render_content(f: &mut ratatui::Frame, editor: &Editor, area: ratatui::layout::Rect) {
    let content = Paragraph::new(editor.get_visible_content()).block(Block::default());
    f.render_widget(content, area);
}

fn render_status_line(f: &mut ratatui::Frame, editor: &Editor, area: ratatui::layout::Rect) {
    let (cursor_line, cursor_column) = editor.get_cursor_screen_position();
    let mode_text = format!(" {} ", editor.get_mode());
    let line_col_text = format!("{}:{} ", cursor_line + 1, cursor_column + 1);

    let available_width = area.width as usize;
    let mode_width = mode_text.len();
    let line_col_width = line_col_text.len();

    let status_text = if available_width > mode_width + line_col_width {
        let padding = " ".repeat(available_width - mode_width - line_col_width);
        format!("{}{}{}", mode_text, padding, line_col_text)
    } else if available_width > mode_width {
        let truncated_line_col = &line_col_text[..available_width - mode_width];
        format!("{}{}", mode_text, truncated_line_col)
    } else {
        mode_text[..available_width.min(mode_text.len())].to_string()
    };

    let status_line =
        Paragraph::new(status_text).style(Style::default().bg(Color::from_u32(0x202020)));

    f.render_widget(status_line, area);
}

fn render_debug_info(f: &mut ratatui::Frame, editor: &Editor, area: ratatui::layout::Rect) {
    if editor.is_debug_info_visible() {
        let debug_info = Paragraph::new(editor.get_debug_info());
        f.render_widget(debug_info, area);
    }
}

fn handle_input(editor: &mut Editor, key: event::KeyEvent) -> Result<bool, Box<dyn Error>> {
    match editor.get_mode() {
        Mode::Normal => handle_normal_mode(editor, key),
        Mode::Insert => handle_insert_mode(editor, key),
    }
}

fn handle_normal_mode(editor: &mut Editor, key: event::KeyEvent) -> Result<bool, Box<dyn Error>> {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q')) => Ok(true),
        (KeyModifiers::NONE, KeyCode::Char('i')) => {
            editor.set_mode(Mode::Insert);
            Ok(false)
        }
        (KeyModifiers::SHIFT, KeyCode::Char('D')) => {
            editor.toggle_debug_info();
            Ok(false)
        }
        (KeyModifiers::NONE, KeyCode::Left) => {
            editor.move_cursor(CursorMovement::Left);
            Ok(false)
        }
        (KeyModifiers::NONE, KeyCode::Right) => {
            editor.move_cursor(CursorMovement::Right);
            Ok(false)
        }
        (KeyModifiers::NONE, KeyCode::Up) => {
            editor.move_cursor(CursorMovement::Up);
            Ok(false)
        }
        (KeyModifiers::NONE, KeyCode::Down) => {
            editor.move_cursor(CursorMovement::Down);
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn handle_insert_mode(editor: &mut Editor, key: event::KeyEvent) -> Result<bool, Box<dyn Error>> {
    match key.code {
        KeyCode::Char(c) => editor.insert(c),
        KeyCode::Backspace => editor.delete(),
        KeyCode::Enter => editor.insert_new_line(),
        KeyCode::Left => editor.move_cursor(CursorMovement::Left),
        KeyCode::Right => editor.move_cursor(CursorMovement::Right),
        KeyCode::Up => editor.move_cursor(CursorMovement::Up),
        KeyCode::Down => editor.move_cursor(CursorMovement::Down),
        KeyCode::Home => editor.move_cursor(CursorMovement::LineStart),
        KeyCode::End => editor.move_cursor(CursorMovement::LineEnd),
        KeyCode::Tab => editor.insert_str("    ".to_string()),
        KeyCode::Esc => editor.set_mode(Mode::Normal),
        _ => {}
    }
    Ok(false)
}
