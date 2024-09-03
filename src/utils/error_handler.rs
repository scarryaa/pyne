use once_cell::sync::Lazy;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};
use std::sync::Mutex;

pub struct ErrorHandler {
    message: Option<String>,
}

impl ErrorHandler {
    pub fn new() -> Self {
        ErrorHandler { message: None }
    }

    pub fn set_error(&mut self, message: String) {
        self.message = Some(message);
    }

    pub fn clear_error(&mut self) {
        self.message = None;
    }

    pub fn get_error(&self) -> Option<&String> {
        self.message.as_ref()
    }
}

pub static ERROR_HANDLER: Lazy<Mutex<ErrorHandler>> = Lazy::new(|| Mutex::new(ErrorHandler::new()));

pub fn render_error(f: &mut Frame, area: Rect) {
    if let Some(error_message) = get_error() {
        let error_paragraph = Paragraph::new(error_message).style(Style::default().fg(Color::Red));
        f.render_widget(error_paragraph, area);
    }
}

pub fn set_error(message: String) {
    ERROR_HANDLER.lock().unwrap().set_error(message);
}

pub fn clear_error() {
    ERROR_HANDLER.lock().unwrap().clear_error();
}

pub fn get_error() -> Option<String> {
    ERROR_HANDLER.lock().unwrap().get_error().cloned()
}
