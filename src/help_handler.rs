use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct HelpHandler {
    topics: HashMap<String, String>,
    current_topic: Option<String>,
}

impl HelpHandler {
    pub fn new() -> Self {
        let mut topics = HashMap::new();
        topics.insert(
            "default".to_string(),
            "Welcome to the help system. Use :help <topic> for specific help.".to_string(),
        );
        topics.insert(
            "commands".to_string(),
            r#"Available commands:
:q - Quit (if no unsaved changes)
:q! - Force quit
:w - Save current file
:w <filename> - Save as <filename>
:wq - Save and quit
:e <filename> - Edit <filename>
:help - Show this help message
:set <option> - Set editor option
:split - Split view horizontally (not implemented)
:vsplit - Split view vertically (not implemented)"#
                .to_string(),
        );

        HelpHandler {
            topics,
            current_topic: None,
        }
    }

    pub fn set_help_topic(&mut self, topic: &str) {
        self.current_topic = Some(topic.to_string());
    }

    pub fn clear_help_topic(&mut self) {
        self.current_topic = None;
    }

    pub fn get_help_text(&self) -> Option<&String> {
        self.current_topic
            .as_ref()
            .and_then(|topic| self.topics.get(topic))
            .or_else(|| self.topics.get("default"))
    }

    pub fn add_topic(&mut self, topic: String, content: String) {
        self.topics.insert(topic, content);
    }
}

pub static HELP_HANDLER: Lazy<Mutex<HelpHandler>> = Lazy::new(|| Mutex::new(HelpHandler::new()));

pub fn set_help_topic(topic: &str) {
    HELP_HANDLER.lock().unwrap().set_help_topic(topic);
}

pub fn clear_help_topic() {
    HELP_HANDLER.lock().unwrap().clear_help_topic();
}

pub fn get_help_text() -> Option<String> {
    HELP_HANDLER.lock().unwrap().get_help_text().cloned()
}

pub fn add_help_topic(topic: String, content: String) {
    HELP_HANDLER.lock().unwrap().add_topic(topic, content);
}

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub fn render_help(f: &mut Frame, area: Rect) {
    if let Some(help_text) = get_help_text() {
        let help_paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::Yellow));
        f.render_widget(help_paragraph, area);
    }
}
