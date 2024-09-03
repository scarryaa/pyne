use command_bar::CommandBar;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use error_handler::{clear_error, set_error};
use file_explorer::FileExplorer;
use pyne::{
    ui::command_bar,
    utils::{error_handler, file_explorer},
};
use ratatui::{
    backend::CrosstermBackend,
    crossterm,
    layout::{Constraint, Direction, Layout, Position},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Terminal,
};
use std::{env, error::Error, io, path::PathBuf};

const SUGGESTIONS_PER_PAGE: usize = 5;

use pyne::editor::cursor_movement::CursorMovement;
use pyne::editor::mode::Mode;
use pyne::editor::Editor;
use pyne::ui::gutter::Gutter;

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
            set_error(format!("Failed to open file: {:?}", err));
        }
    } else if !default_file_path.is_empty() {
        set_error(format!("File does not exist: {:?}", file_path.display()));
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
    let mut command_bar = CommandBar::new();

    loop {
        terminal.draw(|f| render_ui(f, editor, file_explorer, &command_bar))?;

        if let Event::Key(key) = event::read()? {
            if handle_input(editor, file_explorer, &mut command_bar, key)? {
                break;
            }
        }
    }
    Ok(())
}

fn render_ui(
    f: &mut ratatui::Frame,
    editor: &mut Editor,
    file_explorer: &mut FileExplorer,
    command_bar: &CommandBar,
) {
    let area = f.area();
    if file_explorer.open {
        file_explorer.render(f, area);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Editor area
                Constraint::Length(1), // Command description
                Constraint::Length(1), // Status bar / Command bar
                Constraint::Length(1), // Error message
            ])
            .split(area);

        let editor_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(6), Constraint::Min(1)])
            .split(chunks[0]);

        editor.set_viewport((editor_chunks[1].width as usize, chunks[0].height as usize));
        render_gutter(f, editor, editor_chunks[0]);
        render_content(f, editor, editor_chunks[1]);
        render_command_description(f, command_bar, chunks[1]);
        render_status_line(f, editor, command_bar, chunks[2]);
        render_autocomplete_suggestions(f, command_bar, chunks[3]);
        error_handler::render_error(f, chunks[2]);
        // help_handler::render_help(f, chunks[4]);

        // Handle Option types for cursor position and scroll offset
        if let (Some((cursor_line, cursor_column)), Some((scroll_x, scroll_y))) = (
            editor.get_cursor_screen_position(),
            editor.get_scroll_offset(),
        ) {
            let cursor_screen_x = (cursor_column as i32 - scroll_x as i32).max(0) as u16;
            let cursor_screen_y = (cursor_line as i32 - scroll_y as i32).max(0) as u16;
            f.set_cursor_position(Position::new(
                editor_chunks[1].x + cursor_screen_x,
                chunks[0].y + cursor_screen_y,
            ));
        }
    }
}

fn render_command_description(
    f: &mut ratatui::Frame,
    command_bar: &CommandBar,
    area: ratatui::layout::Rect,
) {
    if command_bar.is_active() {
        if let Some(description) = command_bar.get_current_command_description() {
            let description_widget =
                Paragraph::new(description).style(Style::default().fg(Color::Yellow));
            f.render_widget(description_widget, area);
        }
    }
}

fn render_autocomplete_suggestions(
    f: &mut ratatui::Frame,
    command_bar: &CommandBar,
    area: ratatui::layout::Rect,
) {
    if command_bar.is_active() {
        let suggestions = command_bar.get_suggestions();
        let current_index = command_bar.get_suggestion_index();
        let page = command_bar.suggestion_page;
        let total_pages = (suggestions.len() + SUGGESTIONS_PER_PAGE - 1) / SUGGESTIONS_PER_PAGE;

        let start_index = page * SUGGESTIONS_PER_PAGE;
        let end_index = (start_index + SUGGESTIONS_PER_PAGE).min(suggestions.len());

        let mut spans = Vec::new();

        // Add left arrow for previous page
        if page > 0 {
            spans.push(Span::styled("< ", Style::default().fg(Color::Yellow)));
        }

        for (index, suggestion) in suggestions[start_index..end_index].iter().enumerate() {
            let absolute_index = start_index + index;
            if absolute_index == current_index {
                spans.push(Span::styled(
                    suggestion.name.clone(),
                    Style::default().fg(Color::Black).bg(Color::White),
                ));
            } else {
                spans.push(Span::styled(
                    suggestion.name.clone(),
                    Style::default().fg(Color::Blue),
                ));
            }

            if index < end_index - start_index - 1 {
                spans.push(Span::raw(" "));
            }
        }

        // Add right arrow for next page
        if page < total_pages - 1 {
            spans.push(Span::styled(" >", Style::default().fg(Color::Yellow)));
        }

        // Add page indicator
        let page_indicator = format!(" [{}/{}]", page + 1, total_pages);
        spans.push(Span::styled(
            page_indicator,
            Style::default().fg(Color::Gray),
        ));

        let suggestions_line = Line::from(spans);
        let suggestions_widget = Paragraph::new(vec![suggestions_line]);
        f.render_widget(suggestions_widget, area);
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
        let selection = editor.get_selection();
        let lines: Vec<Line> = content
            .lines()
            .enumerate()
            .map(|(line_idx, line)| {
                if let Some((start, end)) = selection {
                    let line_start = editor
                        .get_current_buffer()
                        .unwrap()
                        .content
                        .line_to_char(line_idx);
                    let line_end = line_start + line.len();

                    if line_start < end && line_end > start {
                        let sel_start = start.saturating_sub(line_start);
                        let sel_end = (end - line_start).min(line.len());

                        let mut spans = Vec::new();
                        if sel_start > 0 {
                            spans.push(Span::raw(&line[..sel_start]));
                        }
                        spans.push(Span::styled(
                            &line[sel_start..sel_end],
                            Style::default().bg(Color::Gray).fg(Color::Black),
                        ));
                        if sel_end < line.len() {
                            spans.push(Span::raw(&line[sel_end..]));
                        }
                        Line::from(spans)
                    } else {
                        Line::from(line.to_string())
                    }
                } else {
                    Line::from(line.to_string())
                }
            })
            .collect();

        let paragraph =
            ratatui::widgets::Paragraph::new(lines).block(ratatui::widgets::Block::default());
        f.render_widget(paragraph, area);
    } else {
        let paragraph =
            ratatui::widgets::Paragraph::new("").block(ratatui::widgets::Block::default());
        f.render_widget(paragraph, area);
    }
}

fn render_status_line(
    f: &mut ratatui::Frame,
    editor: &Editor,
    command_bar: &CommandBar,
    area: ratatui::layout::Rect,
) {
    let mode_text = format!(" {} ", editor.get_mode());
    let cursor_info = match editor.get_cursor_screen_position() {
        Some((line, column)) => format!("{}:{} ", line + 1, column + 1),
        None => String::from("No active buffer "),
    };

    let status_text = if command_bar.is_active() {
        format!(":{}", command_bar.get_input())
    } else {
        let available_width = area.width as usize;
        let mode_width = mode_text.len();
        let cursor_info_width = cursor_info.len();

        if available_width > mode_width + cursor_info_width {
            let padding = " ".repeat(available_width - mode_width - cursor_info_width);
            format!("{}{}{}", mode_text, padding, cursor_info)
        } else if available_width > mode_width {
            let truncated_cursor_info = &cursor_info[..available_width - mode_width];
            format!("{}{}", mode_text, truncated_cursor_info)
        } else {
            mode_text[..available_width.min(mode_text.len())].to_string()
        }
    };

    let status_style = Style::default().bg(Color::from_u32(0x202020));
    let status_line = Paragraph::new(status_text).style(status_style);
    f.render_widget(status_line, area);
}

fn handle_input(
    editor: &mut Editor,
    file_explorer: &mut FileExplorer,
    command_bar: &mut CommandBar,
    key: event::KeyEvent,
) -> Result<bool, Box<dyn Error>> {
    clear_error();
    file_explorer.clear_error_message();

    if file_explorer.open {
        handle_file_explorer_input(editor, file_explorer, key)
    } else {
        match editor.get_mode() {
            Mode::Normal => handle_normal_mode(editor, file_explorer, command_bar, key),
            Mode::Insert => handle_insert_mode(editor, key),
            Mode::Visual => handle_visual_mode(editor, key),
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
    command_bar: &mut CommandBar,
    key: event::KeyEvent,
) -> Result<bool, Box<dyn Error>> {
    if command_bar.is_active() {
        match key.code {
            KeyCode::Char(':') if command_bar.get_input().is_empty() => Ok(false),
            KeyCode::Char(c) => {
                command_bar.input(c);
                command_bar.reset_suggestion_index();
                Ok(false)
            }
            KeyCode::Backspace => {
                command_bar.backspace();
                command_bar.reset_suggestion_index();
                Ok(false)
            }
            KeyCode::Tab => {
                command_bar.cycle_suggestion(true);
                Ok(false)
            }
            KeyCode::BackTab => {
                command_bar.cycle_suggestion(false);
                Ok(false)
            }
            KeyCode::Right => {
                command_bar.next_suggestion_page();
                Ok(false)
            }
            KeyCode::Left => {
                command_bar.prev_suggestion_page();
                Ok(false)
            }
            KeyCode::Enter => {
                let result = command_bar.execute_command(editor);
                command_bar.deactivate();
                command_bar.reset_suggestion_index();
                result
            }
            KeyCode::Esc => {
                command_bar.deactivate();
                command_bar.reset_suggestion_index();
                Ok(false)
            }
            _ => Ok(false),
        }
    } else {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('v')) => {
                editor.enter_visual_mode();
                Ok(false)
            }
            (KeyModifiers::NONE, KeyCode::Char(':')) => {
                command_bar.activate();
                command_bar.reset_suggestion_index();
                Ok(false)
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
}

fn handle_visual_mode(editor: &mut Editor, key: event::KeyEvent) -> Result<bool, Box<dyn Error>> {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => {
            editor.exit_visual_mode();
            Ok(false)
        }
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            editor.delete_selection();
            editor.exit_visual_mode();
            Ok(false)
        }
        (KeyModifiers::NONE, KeyCode::Char('y')) => {
            if let Some(selected_text) = editor.copy_selection() {
                editor.copy_to_clipboard(&selected_text)?;
                set_error("Copied to clipboard successfully.".to_string());
            }
            editor.exit_visual_mode();
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
        (KeyModifiers::NONE, KeyCode::Home) => {
            editor.move_cursor(CursorMovement::LineStart);
            Ok(false)
        }
        (KeyModifiers::NONE, KeyCode::End) => {
            editor.move_cursor(CursorMovement::LineEnd);
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
