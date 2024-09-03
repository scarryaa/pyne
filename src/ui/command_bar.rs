use crate::{editor::Editor, utils::error_handler::set_error, utils::help_handler::set_help_topic};
use std::path::PathBuf;

const SUGGESTIONS_PER_PAGE: usize = 5;

pub struct Command {
    pub name: String,
    pub description: String,
    pub action: fn(&mut Editor) -> Result<bool, Box<dyn std::error::Error>>,
    pub help_topic: String,
}

pub struct CommandBar {
    input: String,
    active: bool,
    commands: Vec<Command>,
    suggestion_index: usize,
    pub suggestion_page: usize,
}

impl CommandBar {
    pub fn new() -> Self {
        CommandBar {
            input: String::new(),
            active: false,
            commands: vec![
                Command {
                    name: "q".to_string(),
                    description: "Quit the editor".to_string(),
                    action: |editor| {
                        if editor.has_unsaved_changes() {
                            set_error("Unsaved changes. Use :q! to force quit.".to_string());
                            Ok(false)
                        } else {
                            Ok(true) // Signal to quit the application
                        }
                    },
                    help_topic: "quit".to_string(),
                },
                Command {
                    name: "q!".to_string(),
                    description: "Force quit without saving".to_string(),
                    action: |_| Ok(true), // Force quit
                    help_topic: "force_quit".to_string(),
                },
                Command {
                    name: "w".to_string(),
                    description: "Save the current file".to_string(),
                    action: |editor| {
                        if let Some(path) = editor.get_current_file_path() {
                            editor.save_file(&path)?;
                            set_error("File saved successfully.".to_string());
                        } else {
                            set_error("No file path set. Use :w <filename> to save.".to_string());
                        }
                        Ok(false)
                    },
                    help_topic: "save".to_string(),
                },
                Command {
                    name: "wq".to_string(),
                    description: "Save and quit".to_string(),
                    action: |editor| {
                        if let Some(path) = editor.get_current_file_path() {
                            editor.save_file(&path)?;
                            Ok(true) // Signal to quit after saving
                        } else {
                            set_error(
                                "No file path set. Use :w <filename> to save before quitting."
                                    .to_string(),
                            );
                            Ok(false)
                        }
                    },
                    help_topic: "save_and_quit".to_string(),
                },
                Command {
                    name: "e".to_string(),
                    description: "Edit a file".to_string(),
                    action: |_| {
                        set_error("Use :e <filename> to open a file.".to_string());
                        Ok(false)
                    },
                    help_topic: "edit".to_string(),
                },
                Command {
                    name: "help".to_string(),
                    description: "Show help information".to_string(),
                    action: |_| {
                        set_help_topic("commands");
                        Ok(false)
                    },
                    help_topic: "help".to_string(),
                },
                Command {
                    name: "set".to_string(),
                    description: "Set editor options".to_string(),
                    action: |_| {
                        set_error("Use :set <option> to set an editor option.".to_string());
                        Ok(false)
                    },
                    help_topic: "set_options".to_string(),
                },
                Command {
                    name: "split".to_string(),
                    description: "Split the window horizontally".to_string(),
                    action: |_| {
                        set_error("Split view not implemented yet.".to_string());
                        Ok(false)
                    },
                    help_topic: "split".to_string(),
                },
                Command {
                    name: "vsplit".to_string(),
                    description: "Split the window vertically".to_string(),
                    action: |_| {
                        set_error("Vertical split view not implemented yet.".to_string());
                        Ok(false)
                    },
                    help_topic: "vsplit".to_string(),
                },
            ],
            suggestion_index: 0,
            suggestion_page: 0,
        }
    }

    pub fn get_command(&self) -> Option<&Command> {
        let input = self.input.trim();
        self.commands.iter().find(|cmd| cmd.name == input)
    }

    pub fn get_suggestions(&self) -> Vec<&Command> {
        self.commands.iter().collect()
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
        self.suggestion_index = 0;
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.suggestion_index = 0;
    }

    pub fn get_input(&self) -> &str {
        &self.input
    }

    pub fn get_current_command_description(&self) -> Option<&str> {
        self.get_suggestions()
            .get(self.suggestion_index)
            .map(|cmd| cmd.description.as_str())
    }

    pub fn get_suggestion_index(&self) -> usize {
        self.suggestion_index
    }

    pub fn reset_suggestion_index(&mut self) {
        self.suggestion_index = 0;
    }

    pub fn cycle_suggestion(&mut self, forward: bool) {
        let total_suggestions = self.commands.len();
        if total_suggestions > 0 {
            if forward {
                self.suggestion_index = (self.suggestion_index + 1) % total_suggestions;
            } else {
                self.suggestion_index =
                    (self.suggestion_index + total_suggestions - 1) % total_suggestions;
            }

            // Update page if necessary
            self.suggestion_page = self.suggestion_index / SUGGESTIONS_PER_PAGE;

            // Update the command bar input
            if let Some(cmd) = self.commands.get(self.suggestion_index) {
                self.input = cmd.name.clone();
            }
        }
    }

    pub fn next_suggestion_page(&mut self) {
        let total_pages =
            (self.get_suggestions().len() + SUGGESTIONS_PER_PAGE - 1) / SUGGESTIONS_PER_PAGE;
        self.suggestion_page = (self.suggestion_page + 1) % total_pages;
    }

    pub fn prev_suggestion_page(&mut self) {
        let total_pages =
            (self.get_suggestions().len() + SUGGESTIONS_PER_PAGE - 1) / SUGGESTIONS_PER_PAGE;
        self.suggestion_page = (self.suggestion_page + total_pages - 1) % total_pages;
    }

    pub fn execute_command(&self, editor: &mut Editor) -> Result<bool, Box<dyn std::error::Error>> {
        let input = self.input.trim();

        // Handle commands with arguments
        if input.starts_with("help ") {
            let topic = &input[5..];
            set_help_topic(topic);
            return Ok(false);
        }

        // Handle commands with arguments
        if input.starts_with("w ") {
            let path = PathBuf::from(&input[2..]);
            editor.save_file(&path)?;
            set_error("File saved successfully.".to_string());
            return Ok(false);
        } else if input.starts_with("e ") {
            let path = PathBuf::from(&input[2..]);
            match editor.open_file(&path) {
                Ok(_) => {
                    set_error(format!("Opened file: {:?}", path));
                    return Ok(false);
                }
                Err(e) => {
                    set_error(format!("Failed to open file: {:?}. Error: {}", path, e));
                    return Ok(false);
                }
            }
        } else if input.starts_with("set ") {
            editor.handle_set_command(&input[4..]);
            return Ok(false);
        }

        if let Some(command) = self.get_command() {
            set_help_topic(&command.help_topic);
            return (command.action)(editor);
        }

        set_error(format!("Unknown command: {}", input));
        Ok(false)
    }
}
