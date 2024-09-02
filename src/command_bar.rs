use std::path::PathBuf;

use crate::editor::Editor;

pub struct CommandBar {
    input: String,
    active: bool,
    suggestions: Vec<String>,
}

impl CommandBar {
    pub fn new() -> Self {
        let mut command_bar = CommandBar {
            input: String::new(),
            active: false,
            suggestions: vec![],
        };
        command_bar.update_suggestions();
        command_bar
    }

    pub fn set_input(&mut self, input: String) {
        self.input = input;
        self.update_suggestions();
    }

    pub fn update_suggestions(&mut self) {
        let commands = vec!["q", "q!", "w", "wq", "e", "help", "set", "split", "vsplit"];
        self.suggestions = commands.into_iter().map(String::from).collect();
    }

    pub fn get_suggestions(&self) -> &[String] {
        &self.suggestions
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.input.clear();
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.input.clear();
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn input(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    pub fn get_input(&self) -> &str {
        &self.input
    }

    pub fn take_input(&mut self) -> String {
        std::mem::take(&mut self.input)
    }
}
