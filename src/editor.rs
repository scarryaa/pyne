use std::{io, path::PathBuf};

use crate::{cursor_movement::CursorMovement, mode::Mode};
use ropey::Rope;

pub struct Editor {
    cursor_pos: usize,
    content: Rope,
    mode: Mode,
    viewport: (usize, usize),
    scroll_offset: (usize, usize),
    show_debug_info: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            cursor_pos: 0,
            content: Rope::from_str(""),
            mode: Mode::Normal,
            viewport: (80, 24),
            scroll_offset: (0, 0),
            show_debug_info: false,
        }
    }

    pub fn get_content(&self) -> Rope {
        self.content.clone()
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

    pub fn get_scroll_offset(&self) -> (usize, usize) {
        self.scroll_offset
    }

    pub fn scroll(&mut self) {
        const VERTICAL_PADDING: usize = 6;
        const HORIZONTAL_PADDING: usize = 6;

        // Vertical scroll
        let cursor_line = self.cursor_pos_to_line();
        let (_, viewport_height) = self.viewport;
        let (_, scroll_y) = self.scroll_offset;

        if cursor_line < scroll_y + VERTICAL_PADDING {
            self.scroll_offset.1 = cursor_line.saturating_sub(VERTICAL_PADDING);
        } else if cursor_line >= scroll_y + viewport_height - VERTICAL_PADDING {
            self.scroll_offset.1 =
                cursor_line.saturating_sub(viewport_height - VERTICAL_PADDING - 1);
        }

        // Horizontal scroll
        let cursor_column = self.get_cursor_screen_position().1;
        let (viewport_width, _) = self.viewport;
        let (scroll_x, _) = self.scroll_offset;

        if cursor_column < scroll_x + HORIZONTAL_PADDING {
            self.scroll_offset.0 = cursor_column.saturating_sub(HORIZONTAL_PADDING);
        } else if cursor_column >= scroll_x + viewport_width - HORIZONTAL_PADDING {
            self.scroll_offset.0 =
                cursor_column.saturating_sub(viewport_width - HORIZONTAL_PADDING - 1);
        }
    }

    pub fn cursor_pos_to_char(&self) -> usize {
        self.content.char_to_byte(self.cursor_pos)
    }

    pub fn cursor_pos_to_line(&self) -> usize {
        self.content.char_to_line(self.cursor_pos)
    }

    pub fn insert_str(&mut self, s: String) {
        self.content.insert(self.cursor_pos, &s);
        self.cursor_pos += s.len();
        self.scroll();
    }

    pub fn insert(&mut self, char: char) {
        self.content.insert_char(self.cursor_pos, char);
        self.cursor_pos += 1;
        self.scroll();
    }

    pub fn insert_new_line(&mut self) {
        self.content.insert_char(self.cursor_pos, '\n');
        self.cursor_pos += 1;
        self.scroll();
    }

    pub fn delete(&mut self) {
        if self.cursor_pos > 0 {
            self.content.remove(self.cursor_pos - 1..self.cursor_pos);
            self.cursor_pos -= 1;
            self.scroll();
        }
    }

    pub fn move_cursor(&mut self, direction: CursorMovement) {
        match direction {
            CursorMovement::Left => self.move_cursor_left(),
            CursorMovement::Right => self.move_cursor_right(),
            CursorMovement::Up => self.move_cursor_up(),
            CursorMovement::Down => self.move_cursor_down(),
            CursorMovement::LineStart => self.move_cursor_line_start(),
            CursorMovement::LineEnd => self.move_cursor_line_end(),
        }
        self.scroll();
    }

    pub fn to_string(&self) -> String {
        self.content.to_string()
    }

    pub fn get_visible_content(&self) -> String {
        let (scroll_x, scroll_y) = self.scroll_offset;
        let (viewport_width, viewport_height) = self.viewport;
        let max_line_idx = self.content.len_lines().min(scroll_y + viewport_height);
        (scroll_y..max_line_idx)
            .map(|line_idx| {
                let line = self.content.line(line_idx);
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

                if line_idx == self.cursor_pos_to_line() {
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
    }

    pub fn get_cursor_screen_position(&self) -> (usize, usize) {
        let line = self.content.char_to_line(self.cursor_pos);
        let line_start = self.content.line_to_char(line);
        let column = self.cursor_pos - line_start;
        (line, column)
    }

    fn move_cursor_line_start(&mut self) {
        let line = self.content.char_to_line(self.cursor_pos);
        let line_start = self.content.line_to_char(line);

        self.cursor_pos = line_start;
    }

    fn move_cursor_line_end(&mut self) {
        let line = self.content.char_to_line(self.cursor_pos);
        let line_end = self.content.line_to_char(line + 1).saturating_sub(1);

        self.cursor_pos = line_end;
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.content.len_chars() {
            self.cursor_pos += 1;
        }
    }

    fn move_cursor_up(&mut self) {
        if let Some(prev_line) = self.content.char_to_line(self.cursor_pos).checked_sub(1) {
            let cur_line_start = self
                .content
                .line_to_char(self.content.char_to_line(self.cursor_pos));
            let prev_line_start = self.content.line_to_char(prev_line);
            let cur_col = self.cursor_pos - cur_line_start;
            let prev_line_len = self.content.line(prev_line).len_chars();
            self.cursor_pos = prev_line_start + cur_col.min(prev_line_len);
        }
    }

    fn move_cursor_down(&mut self) {
        let next_line = self.content.char_to_line(self.cursor_pos) + 1;
        if next_line < self.content.len_lines() {
            let cur_line_start = self
                .content
                .line_to_char(self.content.char_to_line(self.cursor_pos));
            let next_line_start = self.content.line_to_char(next_line);
            let cur_col = self.cursor_pos - cur_line_start;
            let next_line_len = self.content.line(next_line).len_chars();
            self.cursor_pos = next_line_start + cur_col.min(next_line_len);
        }
    }

    pub fn open_file(&mut self, path: &PathBuf) -> io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.content = Rope::from_str(&content);
        self.cursor_pos = 0;
        self.scroll_offset = (0, 0);
        Ok(())
    }

    pub fn toggle_debug_info(&mut self) {
        self.show_debug_info = !self.show_debug_info;
    }

    pub fn is_debug_info_visible(&self) -> bool {
        self.show_debug_info
    }

    pub fn get_debug_info(&self) -> String {
        format!(
            "Cursor: {}, Viewport: {:?}, Scroll: {:?}, Content length: {}, Lines: {}",
            self.cursor_pos,
            self.viewport,
            self.scroll_offset,
            self.content.len_chars(),
            self.content.len_lines()
        )
    }
}
