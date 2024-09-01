use crate::{cursor_movement::CursorMovement, mode::Mode};
use ropey::Rope;

pub struct Editor {
    cursor_pos: usize,
    content: Rope,
    mode: Mode,
    viewport: (usize, usize),
    scroll_offset: (usize, usize),
}

impl Editor {
    pub fn new() -> Self {
        Self {
            cursor_pos: 0,
            content: Rope::from_str(""),
            mode: Mode::Normal,
            viewport: (80, 24),
            scroll_offset: (0, 0),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> Mode {
        self.mode.clone()
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
        let mut result = String::new();
        for line_idx in scroll_y..scroll_y + viewport_height {
            if line_idx < self.content.len_lines() {
                let line = self.content.line(line_idx);
                if scroll_x >= line.len_chars() {
                    // If the entire line is scrolled off to the left, add a newline
                    result.push('\n');
                } else {
                    let visible_line: String =
                        line.chars().skip(scroll_x).take(viewport_width).collect();
                    result.push_str(&visible_line);
                }
            }
        }
        result
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
