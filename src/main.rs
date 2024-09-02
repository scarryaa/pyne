use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use file_explorer::FileExplorer;
use ratatui::{
    backend::CrosstermBackend,
    crossterm,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph},
    Terminal,
};
use std::{env, error::Error, io, path::PathBuf};

mod cursor_movement;
mod editor;
mod file_explorer;
mod gutter;
mod mode;

use cursor_movement::CursorMovement;
use editor::Editor;
use gutter::Gutter;
use mode::Mode;

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;
    let mut editor = Editor::new();
    let mut file_explorer = FileExplorer::new(&env::current_dir()?)?;

    // Store the starting directory
    let starting_directory = env::current_dir()?;

    // Determine the file to open
    let default_file_path = env::args().nth(1).unwrap_or_else(|| "".to_string());
    let file_path = PathBuf::from(&default_file_path);

    // Open the file if it exists or initialize a scratch buffer
    if file_path.exists() {
        if let Err(err) = editor.open_file(&file_path) {
            editor.show_error(format!("Failed to open file: {:?}", err));
        }
    } else if !default_file_path.is_empty() {
        editor.show_error(format!("File does not exist: {:?}", file_path.display()));
    } else {
        editor.new_scratch_buffer()?;
        editor.set_starting_directory(starting_directory.clone());
        file_explorer.set_starting_directory(starting_directory.clone());
    }

    // Set the file explorer's directory to the starting directory if it's a scratch buffer
    if editor.is_scratch_buffer() {
        file_explorer.set_current_directory(starting_directory)?;
    } else if let Some(file_dir) = file_path.parent() {
        file_explorer.set_current_directory(file_dir.to_path_buf())?;
    }

    let result = run_app(&mut terminal, &mut editor, &mut file_explorer);

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
    file_explorer: &mut FileExplorer,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| render_ui(f, editor, file_explorer))?;

        if let Event::Key(key) = event::read()? {
            if handle_input(editor, file_explorer, key)? {
                break;
            }
        }
    }
    Ok(())
}

fn render_ui(f: &mut ratatui::Frame, editor: &mut Editor, file_explorer: &mut FileExplorer) {
    let area = f.size();
    if file_explorer.open {
        file_explorer.render(f, area);
    } else {
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

        // Handle Option types for cursor position and scroll offset
        if let (Some((cursor_line, cursor_column)), Some((scroll_x, scroll_y))) = (
            editor.get_cursor_screen_position(),
            editor.get_scroll_offset(),
        ) {
            let cursor_screen_x = (cursor_column as i32 - scroll_x as i32).max(0) as u16;
            let cursor_screen_y = (cursor_line as i32 - scroll_y as i32).max(0) as u16;
            f.set_cursor(
                editor_chunks[1].x + cursor_screen_x,
                chunks[0].y + cursor_screen_y,
            );
        }

        // Render the error message
        if let Some(ref error_message) = editor.error_message {
            let error_area = chunks[2];
            let error_paragraph =
                Paragraph::new(error_message.to_string()).style(Style::default().fg(Color::Red));
            f.render_widget(error_paragraph, error_area);
        }
    }
}

fn render_gutter(f: &mut ratatui::Frame, editor: &Editor, area: ratatui::layout::Rect) {
    let line_numbers = Gutter::get_visible_line_numbers(editor);
    let gutter_content =
        Paragraph::new(line_numbers.join("\n")).style(Style::default().fg(Color::DarkGray));
    f.render_widget(gutter_content, area);
}

fn render_content(f: &mut ratatui::Frame, editor: &Editor, area: ratatui::layout::Rect) {
    if let Some(content) = editor.get_visible_content() {
        let paragraph = Paragraph::new(content).block(Block::default());
        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("").block(Block::default());
        f.render_widget(paragraph, area);
    }
}

fn render_status_line(f: &mut ratatui::Frame, editor: &Editor, area: ratatui::layout::Rect) {
    let mode_text = format!(" {} ", editor.get_mode());
    let cursor_info = match editor.get_cursor_screen_position() {
        Some((line, column)) => format!("{}:{} ", line + 1, column + 1),
        None => String::from("No active buffer "),
    };
    let line_col_text = cursor_info;

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

fn handle_input(
    editor: &mut Editor,
    file_explorer: &mut FileExplorer,
    key: event::KeyEvent,
) -> Result<bool, Box<dyn Error>> {
    editor.clear_error_message();
    file_explorer.clear_error_message();

    if file_explorer.open {
        handle_file_explorer_input(editor, file_explorer, key)
    } else {
        match editor.get_mode() {
            Mode::Normal => handle_normal_mode(editor, file_explorer, key),
            Mode::Insert => handle_insert_mode(editor, key),
        }
    }
}

fn handle_file_explorer_input(
    editor: &mut Editor,
    file_explorer: &mut FileExplorer,
    key: event::KeyEvent,
) -> Result<bool, Box<dyn Error>> {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('/')) if !file_explorer.is_in_search_mode() => {
            file_explorer.enter_search_mode();
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            if let Some(path) = file_explorer.enter_directory()? {
                if file_explorer.is_binary_or_non_utf8(&path)? {
                    file_explorer.show_error(&format!(
                        "Error: Cannot open binary or non-UTF8 file {:?}",
                        path
                    ));
                } else {
                    file_explorer.open = false;
                    editor.open_file(&path)?;
                }
            }
        }
        (KeyModifiers::NONE, KeyCode::Esc) => {
            if file_explorer.is_in_search_mode() {
                file_explorer.exit_search_mode()?;
            } else {
                file_explorer.open = false;
            }
        }
        (KeyModifiers::NONE, KeyCode::Up) => file_explorer.move_selection(-1)?,
        (KeyModifiers::NONE, KeyCode::Down) => file_explorer.move_selection(1)?,
        (KeyModifiers::NONE, KeyCode::Left) => {
            if !file_explorer.is_in_search_mode() {
                file_explorer.go_up()?;
            }
        }
        (KeyModifiers::NONE, KeyCode::Right) => {
            if !file_explorer.is_in_search_mode() {
                if let Some(path) = file_explorer.enter_directory()? {
                    file_explorer.open = false;
                    editor.open_file(&path)?;
                }
            }
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            if file_explorer.is_in_search_mode() {
                file_explorer.handle_search_backspace()?;
            } else {
                file_explorer.go_up()?;
            }
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            file_explorer.toggle_global_search()?;
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            if file_explorer.is_in_search_mode() {
                file_explorer.handle_search_input(c)?;
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_normal_mode(
    editor: &mut Editor,
    file_explorer: &mut FileExplorer,
    key: event::KeyEvent,
) -> Result<bool, Box<dyn Error>> {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q')) => {
            if editor.has_unsaved_changes() {
                let unsaved_buffers = editor.get_unsaved_buffers();
                let error_message = format!(
                    "Error: Unsaved changes in {} buffer(s). Save changes before quitting.",
                    unsaved_buffers.len()
                );
                editor.show_error(error_message);
                Ok(true)
            } else {
                Ok(true)
            }
        }
        (KeyModifiers::NONE, KeyCode::Char('i')) => {
            editor.set_mode(Mode::Insert);
            Ok(false)
        }
        (KeyModifiers::NONE, KeyCode::Char('f')) => {
            file_explorer.open = true;

            if editor.is_scratch_buffer() || editor.get_current_file_path().is_none() {
                if let Some(starting_dir) = editor.get_starting_directory() {
                    file_explorer.set_current_directory(starting_dir.clone())?;
                } else {
                    // Fallback to current directory if starting directory is not set
                    file_explorer.set_current_directory(env::current_dir()?)?;
                }
            } else {
                // Otherwise, open the directory of the current file
                file_explorer
                    .open_current_file_directory(editor.get_current_file_path().as_deref())?;
            }
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
