use std::path::PathBuf;

pub struct Split {
    pub buffer: Option<PathBuf>,
    pub cursor_pos: usize,
    pub scroll_offset: (usize, usize),
}

impl Split {
    pub fn new() -> Self {
        Self {
            buffer: None,
            cursor_pos: 0,
            scroll_offset: (0, 0),
        }
    }
}
