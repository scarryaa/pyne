use ratatui::{
    crossterm::style::SetAttribute,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::{
    cmp::Ordering,
    fs::{self, File},
};

pub struct FileExplorer {
    starting_path: PathBuf,
    current_path: PathBuf,
    entries: Vec<PathBuf>,
    list_state: ListState,
    preview_content: String,
    pub open: bool,
    search_query: String,
    search_mode: bool,
    global_search: bool,
    error_message: Option<String>,
}

impl FileExplorer {
    pub fn new(initial_path: &Path) -> io::Result<Self> {
        let mut explorer = FileExplorer {
            starting_path: initial_path.to_path_buf(),
            current_path: initial_path.to_path_buf(),
            entries: Vec::new(),
            list_state: ListState::default(),
            preview_content: String::new(),
            open: false,
            search_query: String::new(),
            search_mode: false,
            global_search: false,
            error_message: None,
        };
        explorer.refresh_entries()?;
        Ok(explorer)
    }

    pub fn set_starting_directory(&mut self, p: PathBuf) {
        self.starting_path = p;
    }

    pub fn set_current_directory(&mut self, path: PathBuf) -> io::Result<()> {
        self.current_path = fs::canonicalize(path)?;
        self.refresh_entries()
    }

    pub fn open_current_file_directory(&mut self, current_file: Option<&Path>) -> io::Result<()> {
        if let Some(file_path) = current_file {
            if let Some(parent) = file_path.parent() {
                self.current_path = fs::canonicalize(parent)?;
                self.refresh_entries()?;
            }
        }
        self.open = true;
        Ok(())
    }

    pub fn clear_error_message(&mut self) {
        self.error_message = None;
    }

    pub fn show_error(&mut self, message: &str) {
        self.error_message = Some(message.to_string());
    }

    fn get_relative_path(&self) -> String {
        match self.current_path.strip_prefix(&self.starting_path) {
            Ok(rel_path) => rel_path.display().to_string(),
            Err(_) => self.current_path.display().to_string(),
        }
    }

    pub fn is_in_search_mode(&self) -> bool {
        self.search_mode
    }

    pub fn search_empty(&self) -> bool {
        self.search_query.to_string().len() == 0
    }

    pub fn toggle_global_search(&mut self) -> io::Result<()> {
        self.global_search = !self.global_search;
        self.update_search()
    }

    pub fn handle_search_input(&mut self, c: char) -> io::Result<()> {
        if !self.search_mode {
            self.search_mode = true;
            self.search_query.clear();
        }
        self.search_query.push(c);
        self.update_search()
    }

    pub fn handle_search_backspace(&mut self) -> io::Result<()> {
        if self.search_mode {
            self.search_query.pop();
            self.update_search()
        } else {
            Ok(())
        }
    }

    pub fn clear_search(&mut self) -> io::Result<()> {
        self.search_mode = false;
        self.search_query.clear();
        self.global_search = false;
        self.refresh_entries()
    }

    fn update_search(&mut self) -> io::Result<()> {
        self.preview_content.clear();
        if self.search_query.is_empty() {
            self.refresh_entries()
        } else if self.global_search {
            self.perform_global_search()
        } else {
            self.perform_filename_search()
        }
    }

    fn perform_global_search(&mut self) -> io::Result<()> {
        let search_results = self.search_recursive(&self.current_path, &self.search_query)?;
        self.entries = search_results;
        self.list_state.select(Some(0));
        Ok(())
    }

    fn perform_filename_search(&mut self) -> io::Result<()> {
        self.entries = fs::read_dir(&self.current_path)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.to_lowercase().contains(&self.search_query.to_lowercase()))
                    .unwrap_or(false)
            })
            .collect();
        self.list_state.select(Some(0));
        Ok(())
    }

    fn search_recursive(&self, dir: &Path, query: &str) -> io::Result<Vec<PathBuf>> {
        let mut results = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                results.extend(self.search_recursive(&path, query)?);
            } else if path
                .to_str()
                .map(|s| s.to_lowercase().contains(&query.to_lowercase()))
                .unwrap_or(false)
            {
                results.push(path);
            }
        }
        Ok(results)
    }

    fn refresh_entries(&mut self) -> io::Result<()> {
        self.entries.clear();
        for entry in fs::read_dir(&self.current_path)? {
            let entry = entry?;
            self.entries.push(entry.path());
        }

        self.entries.sort_by(|a, b| {
            if a == &self.current_path.join("..") {
                Ordering::Less
            } else if b == &self.current_path.join("..") {
                Ordering::Greater
            } else {
                let a_is_dir = a.is_dir();
                let b_is_dir = b.is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            }
        });

        // Ensure ".." is always the first entry
        if self.current_path.parent().is_some() {
            self.entries.insert(0, self.current_path.join(".."));
        }

        self.list_state.select(Some(0));
        self.update_preview()
    }

    pub fn move_selection(&mut self, delta: i32) -> io::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        let current = self.list_state.selected().unwrap_or(0);
        let new_index = (current as i32 + delta).rem_euclid(self.entries.len() as i32) as usize;
        self.list_state.select(Some(new_index));
        self.update_preview()
    }

    pub fn enter_directory(&mut self) -> io::Result<Option<PathBuf>> {
        let selected_path = self
            .list_state
            .selected()
            .and_then(|index| self.entries.get(index).cloned());

        if let Some(path) = selected_path {
            if path.is_dir() {
                self.current_path = fs::canonicalize(path)?;
                self.refresh_entries()?;
                self.clear_search()?;
                Ok(None)
            } else {
                self.clear_search()?;
                Ok(Some(path))
            }
        } else {
            Ok(None)
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
    }

    pub fn exit_search_mode(&mut self) -> io::Result<()> {
        self.search_mode = false;
        self.clear_search()
    }

    pub fn go_up(&mut self) -> io::Result<()> {
        if let Some(parent) = self.current_path.parent() {
            self.current_path = fs::canonicalize(parent)?;
            self.refresh_entries()?;
        }
        Ok(())
    }

    fn update_preview(&mut self) -> io::Result<()> {
        self.preview_content.clear();

        if let Some(selected_index) = self.list_state.selected() {
            if let Some(selected_path) = self.entries.get(selected_index) {
                if selected_path.is_dir() {
                    match self.read_dir_preview(selected_path) {
                        Ok(content) => self.preview_content = content,
                        Err(e) => self.preview_content = format!("Error reading directory: {}", e),
                    }
                } else if selected_path.is_file() {
                    match self.read_file_preview(selected_path) {
                        Ok(content) => self.preview_content = content,
                        Err(e) => self.preview_content = format!("Error reading file: {}", e),
                    }
                }
            }
        }
        Ok(())
    }

    fn read_dir_preview(&self, path: &Path) -> io::Result<String> {
        let mut content = String::new();
        for entry in fs::read_dir(path)? {
            match entry {
                Ok(entry) => {
                    let entry_path = entry.path();
                    let relative_path = entry_path.strip_prefix(path).unwrap_or(&entry_path);
                    let display = if relative_path.is_dir() {
                        format!("{}/\n", relative_path.display())
                    } else {
                        format!("{}\n", relative_path.display())
                    };
                    content.push_str(&display);
                }
                Err(e) => {
                    content.push_str(&format!("Error reading entry: {}\n", e));
                }
            }
        }
        Ok(content)
    }

    fn read_file_preview(&self, path: &Path) -> io::Result<String> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        if buffer.len() > 1024 {
            buffer.truncate(1024);
        }

        match String::from_utf8(buffer) {
            Ok(content) => Ok(content),
            Err(_) => Ok(format!("Binary file: {} bytes", file.metadata()?.len())),
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Main content
                Constraint::Length(1), // Instruction bar
                Constraint::Length(1), // Error message
            ])
            .split(area);

        let centered_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(5),
                Constraint::Percentage(90),
                Constraint::Percentage(5),
            ])
            .split(main_layout[0])[1]; // Use the middle 90%

        let centered_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(7),
                Constraint::Percentage(85),
                Constraint::Percentage(8),
            ])
            .split(centered_area)[1]; // Use the middle 85%

        let explorer_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Path display
                Constraint::Min(0),    // File list and preview
                Constraint::Length(3), // Search bar
            ])
            .split(centered_area);

        // Render path
        let relative_path = self.get_relative_path();
        let path_display = Paragraph::new(relative_path)
            .style(Style::default().fg(Color::Green))
            .block(Block::default().borders(Borders::ALL).title("Current Path"));
        f.render_widget(path_display, explorer_area[0]);

        let main_area = explorer_area[1];
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)].as_ref())
            .split(main_area);

        self.render_file_list(f, chunks[0]);
        self.render_preview(f, chunks[1]);

        // Render search bar
        let search_mode = if self.global_search {
            "Global"
        } else {
            "Filename"
        };
        let search_bar = Paragraph::new(format!("Search ({search_mode}): {}", self.search_query))
            .style(Style::default().fg(if self.is_in_search_mode() {
                Color::Yellow
            } else {
                Color::White
            }))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(search_bar, explorer_area[2]);

        // Render instruction bar
        let instructions =
    " / - Search | ESC - Exit | ↑↓ - Navigate | ENTER - Select | G - Toggle Global Search | BACKSPACE - Previous Directory";
        let instruction_bar = Paragraph::new(instructions)
            .style(
                Style::default()
                    .bg(Color::from_u32(0x202020))
                    .fg(Color::White),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(instruction_bar, main_layout[1]);

        // Render error message
        if let Some(ref error_message) = self.error_message {
            let error_bar =
                Paragraph::new(error_message.as_str()).style(Style::default().fg(Color::Red));
            f.render_widget(error_bar, main_layout[2]);
        }
    }

    pub fn is_binary_or_non_utf8(&self, path: &Path) -> io::Result<bool> {
        let mut file = File::open(path)?;
        let mut content = Vec::new();

        // Read the entire file content into a vector
        file.read_to_end(&mut content)?;

        // Check if the content is valid UTF-8
        match std::str::from_utf8(&content) {
            Ok(_) => Ok(false),
            Err(_) => Ok(true),
        }
    }
    pub fn close(&mut self) -> io::Result<()> {
        self.open = false;
        self.clear_search()
    }

    fn render_file_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let content = if path == &self.current_path.join("..") {
                    "..".to_string()
                } else {
                    let mut name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    if path.is_dir() {
                        name.push('/');
                    }
                    name
                };
                let style = if path.is_dir() {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                let prefix = if Some(index) == self.list_state.selected() {
                    "> "
                } else {
                    "  "
                };
                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(content, style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Files"))
            .highlight_style(Style::default().fg(Color::Yellow));

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_preview(&self, f: &mut Frame, area: Rect) {
        let preview = Paragraph::new(self.preview_content.as_str())
            .block(Block::default().borders(Borders::ALL).title("Preview"));
        f.render_widget(preview, area);
    }
}
