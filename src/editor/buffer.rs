use ropey::Rope;

pub struct Buffer {
    pub content: Rope,
    pub cursor_pos: usize,
    pub scroll_offset: (usize, usize),
    pub is_modified: bool,
    pub selection_start: Option<usize>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            content: Rope::new(),
            cursor_pos: 0,
            scroll_offset: (0, 0),
            is_modified: false,
            selection_start: None,
        }
    }
}
