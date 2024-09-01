use crate::cursor_movement::CursorMovement;
use ropey::Rope;

pub struct Editor {
    cursor_pos: usize,
    content: Rope,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            cursor_pos: 0,
            content: Rope::from_str(""),
        }
    }

    pub fn cursor_pos_to_char(&self) -> usize {
        self.content.char_to_byte(self.cursor_pos)
    }

    pub fn cursor_pos_to_line(&self) -> usize {
        self.content.char_to_line(self.cursor_pos)
    }

    pub fn insert(&mut self, char: char) {
        self.content.insert_char(self.cursor_pos, char);
        self.cursor_pos += 1;
    }

    pub fn insert_new_line(&mut self) {
        self.content.insert_char(self.cursor_pos, '\n');
        self.cursor_pos += 1;
    }

    pub fn delete(&mut self) {
        if self.cursor_pos > 0 {
            self.content.remove(self.cursor_pos - 1..self.cursor_pos);
            self.cursor_pos -= 1;
        }
    }

    pub fn move_cursor(&mut self, direction: CursorMovement) {
        match direction {
            CursorMovement::LEFT => self.move_cursor_left(),
            CursorMovement::RIGHT => self.move_cursor_right(),
            CursorMovement::UP => self.move_cursor_up(),
            CursorMovement::DOWN => self.move_cursor_down(),
        }
    }

    pub fn to_string(&self) -> String {
        self.content.to_string()
    }

    pub fn get_cursor_screen_position(&self) -> (usize, usize) {
        let line = self.content.char_to_line(self.cursor_pos);
        let line_start = self.content.line_to_char(line);
        let column = self.cursor_pos - line_start;
        (line, column)
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
}
