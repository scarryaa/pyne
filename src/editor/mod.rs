use crate::{
    editor::buffer::Buffer, editor::cursor_movement::CursorMovement, editor::mode::Mode,
    utils::error_handler::set_error,
};
use clipboard::{ClipboardContext, ClipboardProvider};
use ropey::Rope;
use std::{collections::HashMap, env, error::Error, fs, io, path::PathBuf};

mod buffer;
pub mod cursor_movement;
pub mod mode;

pub struct Editor {
    mode: Mode,
    viewport: (usize, usize),
    show_debug_info: bool,
    buffers: HashMap<PathBuf, Buffer>,
    current_buffer: Option<PathBuf>,
    starting_directory: Option<PathBuf>,
    clipboard: Option<ClipboardContext>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            viewport: (80, 24),
            show_debug_info: false,
            buffers: HashMap::new(),
            current_buffer: None,
            starting_directory: None,
            clipboard: ClipboardContext::new().ok(),
        }
    }

    pub fn copy_to_clipboard(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        if let Some(clipboard) = &mut self.clipboard {
            clipboard.set_contents(text.to_owned())?;
            Ok(())
        } else {
            Err("Clipboard not available".into())
        }
    }

    pub fn enter_visual_mode(&mut self) {
        if let Some(buffer) = self.get_current_buffer_mut() {
            buffer.selection_start = Some(buffer.cursor_pos);
            self.set_mode(Mode::Visual);
        }
    }

    pub fn exit_visual_mode(&mut self) {
        if let Some(buffer) = self.get_current_buffer_mut() {
            buffer.selection_start = None;
            self.set_mode(Mode::Normal);
        }
    }

    pub fn get_selection(&self) -> Option<(usize, usize)> {
        self.get_current_buffer().and_then(|buffer| {
            buffer.selection_start.map(|start| {
                let end = buffer.cursor_pos;
                (start.min(end), start.max(end))
            })
        })
    }

    pub fn delete_selection(&mut self) {
        if let Some(buffer) = self.get_current_buffer_mut() {
            if let Some(selection_start) = buffer.selection_start {
                let start = selection_start.min(buffer.cursor_pos);
                let end = selection_start.max(buffer.cursor_pos);
                buffer.content.remove(start..end);
                buffer.cursor_pos = start;
                buffer.is_modified = true;
                buffer.selection_start = None;
            }
        }
        self.set_mode(Mode::Normal);
    }

    pub fn copy_selection(&self) -> Option<String> {
        self.get_current_buffer().and_then(|buffer| {
            self.get_selection()
                .map(|(start, end)| buffer.content.slice(start..end).to_string())
        })
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

    pub fn handle_set_command(&mut self, option: &str) {
        set_error(format!(
            "Setting option: {}. (Not actually implemented)",
            option
        ));
    }

    pub fn get_starting_directory(&self) -> Option<&PathBuf> {
        self.starting_directory.as_ref()
    }

    pub fn set_starting_directory(&mut self, path: PathBuf) {
        self.starting_directory = Some(path);
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
                set_error(format!(
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
            selection_start: Some(0),
        };
        self.buffers.insert(resolved_path.clone(), buffer);
        self.current_buffer = Some(resolved_path);
        Ok(())
    }

    pub fn toggle_debug_info(&mut self) {
        self.show_debug_info = !self.show_debug_info;
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

    pub fn get_current_buffer(&self) -> Option<&Buffer> {
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
