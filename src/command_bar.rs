pub struct Command {
    pub name: String,
    pub description: String,
}

pub struct CommandBar {
    input: String,
    active: bool,
    suggestions: Vec<Command>,
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
        let all_commands = vec![
            Command {
                name: "q".to_string(),
                description: "Quit the editor".to_string(),
            },
            Command {
                name: "q!".to_string(),
                description: "Force quit without saving".to_string(),
            },
            Command {
                name: "w".to_string(),
                description: "Save the current file".to_string(),
            },
            Command {
                name: "wq".to_string(),
                description: "Save and quit".to_string(),
            },
            Command {
                name: "e".to_string(),
                description: "Edit a file".to_string(),
            },
            Command {
                name: "help".to_string(),
                description: "Show help information".to_string(),
            },
            Command {
                name: "set".to_string(),
                description: "Set editor options".to_string(),
            },
            Command {
                name: "split".to_string(),
                description: "Split the window horizontally".to_string(),
            },
            Command {
                name: "vsplit".to_string(),
                description: "Split the window vertically".to_string(),
            },
        ];

        self.suggestions = all_commands.into_iter().collect();
    }

    pub fn get_suggestions(&self) -> &[Command] {
        &self.suggestions
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.input.clear();
        self.update_suggestions();
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
        self.update_suggestions();
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.update_suggestions();
    }

    pub fn get_input(&self) -> &str {
        &self.input
    }

    pub fn take_input(&mut self) -> String {
        std::mem::take(&mut self.input)
    }
}
