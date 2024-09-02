use crate::{command_bar::CommandBar, cursor_movement::CursorMovement, mode::Mode};
use ropey::Rope;
use std::{collections::HashMap, env, error::Error, fs, io, path::PathBuf};

pub struct Buffer {
    content: Rope,
    cursor_pos: usize,
    scroll_offset: (usize, usize),
    is_modified: bool,
}

impl Buffer {
    fn new() -> Self {
        Self {
            content: Rope::new(),
            cursor_pos: 0,
            scroll_offset: (0, 0),
            is_modified: false,
        }
    }
}

pub struct Editor {
    mode: Mode,
    viewport: (usize, usize),
    show_debug_info: bool,
    pub error_message: Option<String>,
    buffers: HashMap<PathBuf, Buffer>,
    current_buffer: Option<PathBuf>,
    starting_directory: Option<PathBuf>,
    command_bar: CommandBar,
    original_command_input: String,
    suggestion_index: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            viewport: (80, 24),
            show_debug_info: false,
            error_message: None,
            buffers: HashMap::new(),
            current_buffer: None,
            starting_directory: None,
            command_bar: CommandBar::new(),
            original_command_input: "".to_string(),
            suggestion_index: 0,
        }
    }

    pub fn get_suggestion_index(&self) -> usize {
        self.suggestion_index
    }

    pub fn set_suggestion_index(&mut self, index: usize) {
        self.suggestion_index = index;
    }

    pub fn reset_suggestion_index(&mut self) {
        self.suggestion_index = 0;
    }

    pub fn cycle_suggestion(&mut self, forward: bool) {
        let suggestions = self.get_command_bar_suggestions();
        if !suggestions.is_empty() {
            if forward {
                self.suggestion_index = (self.suggestion_index + 1) % suggestions.len();
            } else {
                self.suggestion_index =
                    (self.suggestion_index + suggestions.len() - 1) % suggestions.len();
            }
            // Update the command bar input, but keep the original input
            self.original_command_input = self.command_bar.get_input().to_string();
            let new_input = format!("{}", suggestions[self.suggestion_index]);
            self.command_bar.set_input(new_input);
        }
    }

    pub fn reset_to_original_input(&mut self) {
        self.command_bar
            .set_input(self.original_command_input.clone());
        self.suggestion_index = 0;
    }

    pub fn command_bar_input(&mut self, c: char) {
        self.command_bar.input(c);
        self.original_command_input = self.command_bar.get_input().to_string();
        self.command_bar.update_suggestions();
        self.suggestion_index = 0;
    }

    pub fn command_bar_backspace(&mut self) {
        self.command_bar.backspace();
        self.original_command_input = self.command_bar.get_input().to_string();
        self.command_bar.update_suggestions();
        self.suggestion_index = 0;
    }

    pub fn save_file(&mut self, path: &PathBuf) -> io::Result<()> {
        if let Some(buffer) = self.get_current_buffer_mut() {
            let content = buffer.content.to_string();
            fs::write(path, content)?;
            buffer.is_modified = false;

            // Update the current buffer path if it's a new file
            if self.current_buffer.is_none() || self.current_buffer.as_ref().unwrap() != path {
                self.current_buffer = Some(path.clone());
            }

            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "No active buffer to save",
            ))
        }
    }

    pub fn get_command_bar_suggestions(&self) -> Vec<String> {
        self.command_bar.get_suggestions().to_vec()
    }

    pub fn set_command_bar_input(&mut self, input: String) {
        self.command_bar.set_input(input);
        self.command_bar.update_suggestions();
    }

    pub fn command_bar_activate(&mut self) {
        self.command_bar.activate();
        self.command_bar.update_suggestions();
    }

    pub fn command_bar_deactivate(&mut self) {
        self.command_bar.deactivate();
    }

    pub fn is_command_bar_active(&self) -> bool {
        self.command_bar.is_active()
    }

    pub fn get_command_bar_input(&self) -> &str {
        self.command_bar.get_input()
    }

    pub fn execute_command(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let command = self.command_bar.take_input();
        let result = self.process_command(&command);
        self.command_bar.deactivate();
        result
    }

    fn process_command(&mut self, command: &str) -> Result<bool, Box<dyn std::error::Error>> {
        match command {
            "q" => {
                if self.has_unsaved_changes() {
                    self.show_error("Unsaved changes. Use :q! to force quit.".to_string());
                    Ok(false)
                } else {
                    Ok(true) // Signal to quit the application
                }
            }
            "q!" => Ok(true), // Force quit
            "w" => {
                if let Some(path) = self.get_current_file_path() {
                    self.save_file(&path)?;
                    self.show_error("File saved successfully.".to_string());
                } else {
                    self.show_error("No file path set. Use :w <filename> to save.".to_string());
                }
                Ok(false)
            }
            cmd if cmd.starts_with("w ") => {
                let path = PathBuf::from(&cmd[2..]);
                self.save_file(&path)?;
                self.show_error("File saved successfully.".to_string());
                Ok(false)
            }
            _ => {
                self.show_error(format!("Unknown command: {}", command));
                Ok(false)
            }
        }
    }

    pub fn get_starting_directory(&self) -> Option<&PathBuf> {
        self.starting_directory.as_ref()
    }

    pub fn set_starting_directory(&mut self, path: PathBuf) {
        self.starting_directory = Some(path);
    }

    pub fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    pub fn get_current_file_path(&self) -> Option<PathBuf> {
        self.current_buffer.clone()
    }

    pub fn get_content(&self) -> Option<Rope> {
        self.current_buffer
            .as_ref()
            .and_then(|path| self.buffers.get(path))
            .map(|buffer| buffer.content.clone())
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> Mode {
        self.mode.clone()
    }

    pub fn get_viewport(&self) -> (usize, usize) {
        self.viewport
    }

    pub fn set_viewport(&mut self, viewport: (usize, usize)) {
        self.viewport = viewport;
        self.scroll();
    }

    pub fn get_scroll_offset(&self) -> Option<(usize, usize)> {
        self.current_buffer
            .as_ref()
            .and_then(|path| self.buffers.get(path))
            .map(|buffer| buffer.scroll_offset)
    }

    pub fn scroll(&mut self) {
        const VERTICAL_PADDING: usize = 6;
        const HORIZONTAL_PADDING: usize = 6;

        let cursor_position = self.get_cursor_screen_position();
        let (viewport_width, viewport_height) = self.viewport;

        if let Some(buffer) = self.get_current_buffer_mut() {
            let cursor_line = buffer.content.char_to_line(buffer.cursor_pos);
            let (scroll_x, scroll_y) = buffer.scroll_offset;

            // Vertical scrolling
            if cursor_line < scroll_y + VERTICAL_PADDING {
                buffer.scroll_offset.1 = cursor_line.saturating_sub(VERTICAL_PADDING);
            } else if cursor_line >= scroll_y + viewport_height - VERTICAL_PADDING {
                buffer.scroll_offset.1 =
                    cursor_line.saturating_sub(viewport_height - VERTICAL_PADDING - 1);
            }

            // Horizontal scrolling
            if let Some((_, cursor_column)) = cursor_position {
                if cursor_column < scroll_x + HORIZONTAL_PADDING {
                    buffer.scroll_offset.0 = cursor_column.saturating_sub(HORIZONTAL_PADDING);
                } else if cursor_column >= scroll_x + viewport_width - HORIZONTAL_PADDING {
                    buffer.scroll_offset.0 =
                        cursor_column.saturating_sub(viewport_width - HORIZONTAL_PADDING - 1);
                }
            }
        }
    }

    pub fn cursor_pos_to_char(&self) -> Option<usize> {
        self.get_current_buffer()
            .map(|buffer| buffer.content.char_to_byte(buffer.cursor_pos))
    }

    pub fn cursor_pos_to_line(&self) -> Option<usize> {
        self.get_current_buffer()
            .map(|buffer| buffer.content.char_to_line(buffer.cursor_pos))
    }

    pub fn insert_str(&mut self, s: String) {
        if let Some(buffer) = self.get_current_buffer_mut() {
            buffer.content.insert(buffer.cursor_pos, &s);
            buffer.cursor_pos += s.len();
            buffer.is_modified = true;
            self.scroll();
        }
    }

    pub fn insert(&mut self, char: char) {
        if let Some(buffer) = self.get_current_buffer_mut() {
            buffer.content.insert_char(buffer.cursor_pos, char);
            buffer.cursor_pos += 1;
            buffer.is_modified = true;
            self.scroll();
        }
    }

    pub fn insert_new_line(&mut self) {
        if let Some(buffer) = self.get_current_buffer_mut() {
            buffer.content.insert_char(buffer.cursor_pos, '\n');
            buffer.cursor_pos += 1;
            buffer.is_modified = true;
            self.scroll();
        }
    }

    pub fn delete(&mut self) {
        if let Some(buffer) = self.get_current_buffer_mut() {
            if buffer.cursor_pos > 0 {
                buffer
                    .content
                    .remove(buffer.cursor_pos - 1..buffer.cursor_pos);
                buffer.cursor_pos -= 1;
                buffer.is_modified = true;
                self.scroll();
            }
        }
    }

    pub fn move_cursor(&mut self, direction: CursorMovement) {
        if let Some(buffer) = self.get_current_buffer_mut() {
            match direction {
                CursorMovement::Left => Self::move_cursor_left(buffer),
                CursorMovement::Right => Self::move_cursor_right(buffer),
                CursorMovement::Up => Self::move_cursor_up(buffer),
                CursorMovement::Down => Self::move_cursor_down(buffer),
                CursorMovement::LineStart => Self::move_cursor_line_start(buffer),
                CursorMovement::LineEnd => Self::move_cursor_line_end(buffer),
            }
            self.scroll();
        }
    }

    pub fn to_string(&self) -> Option<String> {
        self.get_current_buffer()
            .map(|buffer| buffer.content.to_string())
    }

    pub fn get_visible_content(&self) -> Option<String> {
        self.get_current_buffer().map(|buffer| {
            let (scroll_x, scroll_y) = buffer.scroll_offset;
            let (viewport_width, viewport_height) = self.viewport;
            let max_line_idx = buffer.content.len_lines().min(scroll_y + viewport_height);
            (scroll_y..max_line_idx)
                .map(|line_idx| {
                    let line = buffer.content.line(line_idx);
                    let line_length = line.len_chars();
                    let visible_start = scroll_x.min(line_length.saturating_sub(1));
                    let visible_end = (scroll_x + viewport_width).min(line_length);

                    let visible_line: String = if line_length > 0 {
                        line.chars()
                            .skip(visible_start)
                            .take(viewport_width)
                            .collect()
                    } else {
                        String::new()
                    };

                    let has_newline = line.chars().skip(visible_end).any(|c| c == '\n');

                    if line_idx == buffer.content.char_to_line(buffer.cursor_pos) {
                        if has_newline {
                            format!("{}\n", visible_line)
                        } else {
                            visible_line
                        }
                    } else if scroll_x >= line_length.saturating_sub(1) {
                        String::from("\n")
                    } else if has_newline {
                        format!("{}\n", visible_line)
                    } else {
                        format!("{}\n", visible_line.trim_end())
                    }
                })
                .collect()
        })
    }

    pub fn new_scratch_buffer(&mut self) -> Result<(), Box<dyn Error>> {
        // Use the starting directory if it is set
        let config_dir = self
            .starting_directory
            .clone()
            .unwrap_or_else(|| Self::get_config_dir().unwrap_or_else(|| PathBuf::from("config")));

        // Ensure the configuration directory exists
        if !config_dir.exists() {
            if let Err(e) = fs::create_dir_all(&config_dir) {
                self.show_error(format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                ));
                return Err(Box::new(e));
            }
        }

        let path = config_dir.join(format!("scratch_{}.txt", uuid::Uuid::new_v4()));

        let buffer = Buffer::new();
        self.buffers.insert(path.clone(), buffer);
        self.current_buffer = Some(path);

        Ok(())
    }

    pub fn is_scratch_buffer(&self) -> bool {
        self.current_buffer
            .as_ref()
            .map(|path| {
                path.file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("")
                    .starts_with("scratch_")
            })
            .unwrap_or(false)
    }

    fn get_config_dir() -> Option<PathBuf> {
        if let Ok(home) = env::var("HOME") {
            let mut config_path = PathBuf::from(home);
            config_path.push(".config/pyne");
            Some(config_path)
        } else if let Ok(appdata) = env::var("APPDATA") {
            let mut config_path = PathBuf::from(appdata);
            config_path.push("pyne");
            Some(config_path)
        } else {
            None
        }
    }

    pub fn show_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    pub fn get_cursor_screen_position(&self) -> Option<(usize, usize)> {
        self.get_current_buffer().map(|buffer| {
            let line = buffer.content.char_to_line(buffer.cursor_pos);
            let line_start = buffer.content.line_to_char(line);
            let column = buffer.cursor_pos - line_start;
            (line, column)
        })
    }

    pub fn open_file(&mut self, path: &PathBuf) -> io::Result<()> {
        let resolved_path = if path.is_relative() {
            self.starting_directory
                .as_ref()
                .map(|dir| dir.join(path))
                .unwrap_or_else(|| path.clone())
        } else {
            path.clone()
        };

        let content = fs::read_to_string(&resolved_path)?;
        let buffer = Buffer {
            content: Rope::from_str(&content),
            cursor_pos: 0,
            scroll_offset: (0, 0),
            is_modified: false,
        };
        self.buffers.insert(resolved_path.clone(), buffer);
        self.current_buffer = Some(resolved_path);
        Ok(())
    }

    pub fn toggle_debug_info(&mut self) {
        self.show_debug_info = !self.show_debug_info;
    }

    pub fn is_debug_info_visible(&self) -> bool {
        self.show_debug_info
    }

    pub fn get_debug_info(&self) -> String {
        if let Some(buffer) = self.get_current_buffer() {
            format!(
                "Cursor: {}, Viewport: {:?}, Scroll: {:?}, Content length: {}, Lines: {}",
                buffer.cursor_pos,
                self.viewport,
                buffer.scroll_offset,
                buffer.content.len_chars(),
                buffer.content.len_lines()
            )
        } else {
            String::from("No active buffer")
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.buffers.values().any(|buffer| buffer.is_modified)
    }

    pub fn get_unsaved_buffers(&self) -> Vec<PathBuf> {
        self.buffers
            .iter()
            .filter(|(_, buffer)| buffer.is_modified)
            .map(|(path, _)| path.clone())
            .collect()
    }

    fn get_current_buffer(&self) -> Option<&Buffer> {
        self.current_buffer
            .as_ref()
            .and_then(|path| self.buffers.get(path))
    }

    fn get_current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        self.current_buffer
            .as_ref()
            .and_then(|path| self.buffers.get_mut(path))
    }

    fn move_cursor_left(buffer: &mut Buffer) {
        if buffer.cursor_pos > 0 {
            buffer.cursor_pos -= 1;
        }
    }

    fn move_cursor_right(buffer: &mut Buffer) {
        if buffer.cursor_pos < buffer.content.len_chars() {
            buffer.cursor_pos += 1;
        }
    }

    fn move_cursor_up(buffer: &mut Buffer) {
        if let Some(prev_line) = buffer
            .content
            .char_to_line(buffer.cursor_pos)
            .checked_sub(1)
        {
            let cur_line_start = buffer
                .content
                .line_to_char(buffer.content.char_to_line(buffer.cursor_pos));
            let prev_line_start = buffer.content.line_to_char(prev_line);
            let cur_col = buffer.cursor_pos - cur_line_start;
            let prev_line_len = buffer.content.line(prev_line).len_chars();
            buffer.cursor_pos = prev_line_start + cur_col.min(prev_line_len);
        }
    }

    fn move_cursor_down(buffer: &mut Buffer) {
        let next_line = buffer.content.char_to_line(buffer.cursor_pos) + 1;
        if next_line < buffer.content.len_lines() {
            let cur_line_start = buffer
                .content
                .line_to_char(buffer.content.char_to_line(buffer.cursor_pos));
            let next_line_start = buffer.content.line_to_char(next_line);
            let cur_col = buffer.cursor_pos - cur_line_start;
            let next_line_len = buffer.content.line(next_line).len_chars();
            buffer.cursor_pos = next_line_start + cur_col.min(next_line_len);
        }
    }

    fn move_cursor_line_start(buffer: &mut Buffer) {
        let line = buffer.content.char_to_line(buffer.cursor_pos);
        buffer.cursor_pos = buffer.content.line_to_char(line);
    }

    fn move_cursor_line_end(buffer: &mut Buffer) {
        let line = buffer.content.char_to_line(buffer.cursor_pos);
        buffer.cursor_pos = buffer.content.line_to_char(line + 1).saturating_sub(1);
    }
}
