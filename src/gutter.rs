use crate::editor::Editor;

pub struct Gutter;

impl Gutter {
    pub fn get_visible_line_numbers(editor: &Editor) -> Vec<String> {
        let (_, scroll_y) = match editor.get_scroll_offset() {
            Some((x, y)) => (x, y),
            None => (0, 0),
        };

        let (_, viewport_height) = match editor.get_viewport() {
            (width, height) => (width, height),
        };

        let content = editor.get_content().unwrap_or_default();
        let total_lines = content.len_lines();
        let start_line = scroll_y;
        let end_line = (scroll_y + viewport_height).min(total_lines);

        let mut line_numbers = Vec::with_capacity(viewport_height);

        for line_idx in start_line..end_line {
            line_numbers.push(format!("{:>4}", line_idx + 1));
        }

        for _ in end_line..(scroll_y + viewport_height) {
            line_numbers.push("    ".to_string());
        }

        line_numbers
    }
}
