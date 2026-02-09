// Quake-style in-emulator console
//
// Provides a drop-down console overlay that can be toggled with backtick (`)
// Supports command history, tab completion, and persistent output.

use crate::command::{Command, CommandResult};
use std::collections::VecDeque;

/// Maximum lines of output to keep in history
const MAX_OUTPUT_LINES: usize = 100;

/// Maximum command history entries
const MAX_HISTORY: usize = 50;

/// Console overlay for the emulator
pub struct Console {
    /// Is the console currently visible?
    pub visible: bool,

    /// Current input buffer
    input_buffer: String,

    /// Cursor position in input buffer
    cursor_pos: usize,

    /// Command history
    history: Vec<String>,

    /// Current position in history (for up/down navigation)
    history_pos: Option<usize>,

    /// Saved input when navigating history
    saved_input: String,

    /// Output lines (most recent at end)
    output_lines: VecDeque<String>,

    /// Current completions (if any)
    completions: Vec<String>,

    /// Current completion index
    completion_idx: usize,
}

impl Console {
    pub fn new() -> Self {
        Console {
            visible: false,
            input_buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_pos: None,
            saved_input: String::new(),
            output_lines: VecDeque::new(),
            completions: Vec::new(),
            completion_idx: 0,
        }
    }

    /// Toggle console visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Handle text input from SDL
    pub fn handle_text_input(&mut self, text: &str) {
        // Clear completions on new input
        self.completions.clear();

        // Insert text at cursor position
        for ch in text.chars() {
            // Skip control characters and backtick (toggle key)
            if ch.is_control() || ch == '`' {
                continue;
            }
            self.input_buffer.insert(self.cursor_pos, ch);
            self.cursor_pos += ch.len_utf8();
        }
    }

    /// Handle special key presses
    /// Returns Some(command_text) if Enter was pressed, None otherwise
    pub fn handle_key(&mut self, key: ConsoleKey) -> Option<String> {
        match key {
            ConsoleKey::Enter => {
                let input = self.input_buffer.trim().to_string();
                if !input.is_empty() {
                    // Add to history (avoid duplicates at end)
                    if self.history.last() != Some(&input) {
                        self.history.push(input.clone());
                        if self.history.len() > MAX_HISTORY {
                            self.history.remove(0);
                        }
                    }
                }
                self.input_buffer.clear();
                self.cursor_pos = 0;
                self.history_pos = None;
                self.completions.clear();
                if input.is_empty() {
                    None
                } else {
                    Some(input)
                }
            }
            ConsoleKey::Backspace => {
                if self.cursor_pos > 0 {
                    // Find the start of the previous character
                    let prev_char_boundary = self.input_buffer[..self.cursor_pos]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input_buffer.remove(prev_char_boundary);
                    self.cursor_pos = prev_char_boundary;
                }
                self.completions.clear();
                None
            }
            ConsoleKey::Delete => {
                if self.cursor_pos < self.input_buffer.len() {
                    self.input_buffer.remove(self.cursor_pos);
                }
                self.completions.clear();
                None
            }
            ConsoleKey::Left => {
                if self.cursor_pos > 0 {
                    // Move to start of previous character
                    self.cursor_pos = self.input_buffer[..self.cursor_pos]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
                None
            }
            ConsoleKey::Right => {
                if self.cursor_pos < self.input_buffer.len() {
                    // Move past current character
                    let ch = self.input_buffer[self.cursor_pos..].chars().next();
                    if let Some(c) = ch {
                        self.cursor_pos += c.len_utf8();
                    }
                }
                None
            }
            ConsoleKey::Home => {
                self.cursor_pos = 0;
                None
            }
            ConsoleKey::End => {
                self.cursor_pos = self.input_buffer.len();
                None
            }
            ConsoleKey::Up => {
                if !self.history.is_empty() {
                    match self.history_pos {
                        None => {
                            // First press - save current input and go to most recent
                            self.saved_input = self.input_buffer.clone();
                            self.history_pos = Some(self.history.len() - 1);
                        }
                        Some(pos) if pos > 0 => {
                            self.history_pos = Some(pos - 1);
                        }
                        _ => {}
                    }
                    if let Some(pos) = self.history_pos {
                        self.input_buffer = self.history[pos].clone();
                        self.cursor_pos = self.input_buffer.len();
                    }
                }
                self.completions.clear();
                None
            }
            ConsoleKey::Down => {
                if let Some(pos) = self.history_pos {
                    if pos < self.history.len() - 1 {
                        self.history_pos = Some(pos + 1);
                        self.input_buffer = self.history[pos + 1].clone();
                    } else {
                        // At end of history - restore saved input
                        self.history_pos = None;
                        self.input_buffer = self.saved_input.clone();
                    }
                    self.cursor_pos = self.input_buffer.len();
                }
                self.completions.clear();
                None
            }
            ConsoleKey::Tab => {
                self.handle_tab_completion();
                None
            }
            ConsoleKey::Escape => {
                // Clear input or close console
                if !self.input_buffer.is_empty() {
                    self.input_buffer.clear();
                    self.cursor_pos = 0;
                    self.completions.clear();
                } else {
                    self.visible = false;
                }
                None
            }
        }
    }

    /// Handle tab completion
    fn handle_tab_completion(&mut self) {
        if self.completions.is_empty() {
            // Get new completions
            self.completions = Command::complete(&self.input_buffer);
            self.completion_idx = 0;
        } else {
            // Cycle through existing completions
            self.completion_idx = (self.completion_idx + 1) % self.completions.len();
        }

        // Apply current completion
        if !self.completions.is_empty() {
            self.input_buffer = self.completions[self.completion_idx].clone();
            self.cursor_pos = self.input_buffer.len();
        }
    }

    /// Add a line to the output
    pub fn print(&mut self, text: &str) {
        // Split on newlines and add each line
        for line in text.lines() {
            self.output_lines.push_back(line.to_string());
        }
        // Trim to max size
        while self.output_lines.len() > MAX_OUTPUT_LINES {
            self.output_lines.pop_front();
        }
    }

    /// Add command result to output
    pub fn print_result(&mut self, result: &CommandResult) {
        match result {
            CommandResult::Ok { message } => {
                if let Some(msg) = message {
                    self.print(msg);
                }
            }
            CommandResult::Error { message } => {
                self.print(&format!("Error: {}", message));
            }
        }
    }

    /// Get the input prompt string (including cursor)
    pub fn get_input_line(&self) -> String {
        format!("> {}", self.input_buffer)
    }

    /// Get cursor position in the rendered line (accounting for prompt)
    pub fn get_cursor_column(&self) -> usize {
        2 + self.cursor_pos // "> " is 2 chars
    }

    /// Get output lines for rendering
    pub fn get_output_lines(&self) -> impl Iterator<Item = &String> {
        self.output_lines.iter()
    }

    /// Render console to a pixel buffer
    /// This draws directly to the framebuffer overlay
    ///
    /// Parameters:
    /// - pixels: RGB pixel buffer (width * height * 3 bytes)
    /// - width: buffer width in pixels
    /// - height: buffer height in pixels
    /// - console_height: height of console in pixels (from top)
    pub fn render(&self, pixels: &mut [u8], width: usize, height: usize, console_height: usize) {
        if !self.visible {
            return;
        }

        let console_height = console_height.min(height);

        // Draw semi-transparent background (darken existing pixels)
        for y in 0..console_height {
            for x in 0..width {
                let idx = (y * width + x) * 3;
                if idx + 2 < pixels.len() {
                    pixels[idx] = pixels[idx] / 3; // R
                    pixels[idx + 1] = pixels[idx + 1] / 3; // G
                    pixels[idx + 2] = pixels[idx + 2] / 3; // B
                }
            }
        }

        // Character dimensions (8x8 pixel font)
        const CHAR_WIDTH: usize = 6;
        const CHAR_HEIGHT: usize = 8;
        const PADDING: usize = 4;

        // Calculate how many lines we can show
        let max_lines = (console_height - PADDING * 2) / CHAR_HEIGHT;
        if max_lines == 0 {
            return;
        }

        // Reserve one line for input
        let output_lines_to_show = max_lines.saturating_sub(1);

        // Draw output lines (most recent at bottom, above input line)
        let start_line = self.output_lines.len().saturating_sub(output_lines_to_show);
        for (i, line) in self.output_lines.iter().skip(start_line).enumerate() {
            let y = PADDING + i * CHAR_HEIGHT;
            if y + CHAR_HEIGHT > console_height - CHAR_HEIGHT - PADDING {
                break;
            }
            Self::draw_text(pixels, width, PADDING, y, line, [192, 192, 192]);
        }

        // Draw input line at bottom
        let input_y = console_height - CHAR_HEIGHT - PADDING;
        let input_line = self.get_input_line();
        Self::draw_text(pixels, width, PADDING, input_y, &input_line, [255, 255, 255]);

        // Draw cursor
        let cursor_x = PADDING + self.get_cursor_column() * CHAR_WIDTH;
        Self::draw_cursor(pixels, width, cursor_x, input_y, CHAR_HEIGHT);
    }

    /// Draw text using a simple bitmap font
    fn draw_text(
        pixels: &mut [u8],
        width: usize,
        x: usize,
        y: usize,
        text: &str,
        color: [u8; 3],
    ) {
        const CHAR_WIDTH: usize = 6;

        for (i, ch) in text.chars().enumerate() {
            let char_x = x + i * CHAR_WIDTH;
            Self::draw_char(pixels, width, char_x, y, ch, color);
        }
    }

    /// Draw a single character using the built-in font
    fn draw_char(
        pixels: &mut [u8],
        width: usize,
        x: usize,
        y: usize,
        ch: char,
        color: [u8; 3],
    ) {
        // Get font data for character
        let font_data = get_char_bitmap(ch);

        for row in 0..8 {
            let bits = font_data[row];
            for col in 0..6 {
                // Font data uses bits 6-1 (character centered in byte)
                if (bits >> (6 - col)) & 1 != 0 {
                    let px = x + col;
                    let py = y + row;
                    let idx = (py * width + px) * 3;
                    if idx + 2 < pixels.len() {
                        pixels[idx] = color[0];
                        pixels[idx + 1] = color[1];
                        pixels[idx + 2] = color[2];
                    }
                }
            }
        }
    }

    /// Draw blinking cursor
    fn draw_cursor(pixels: &mut [u8], width: usize, x: usize, y: usize, height: usize) {
        // Just draw a vertical bar (no actual blinking - that would need timing)
        for row in 0..height {
            let idx = ((y + row) * width + x) * 3;
            if idx + 2 < pixels.len() {
                pixels[idx] = 255;
                pixels[idx + 1] = 255;
                pixels[idx + 2] = 255;
            }
        }
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

/// Special key events for the console
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsoleKey {
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Up,
    Down,
    Tab,
    Escape,
}

/// Simple 6x8 bitmap font for console rendering
/// Returns 8 bytes, each representing a row of the character (top to bottom)
/// Each row has 6 significant bits (MSB = leftmost pixel)
fn get_char_bitmap(ch: char) -> [u8; 8] {
    match ch {
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '!' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x00, 0x10, 0x00],
        '"' => [0x28, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '#' => [0x28, 0x7C, 0x28, 0x28, 0x7C, 0x28, 0x00, 0x00],
        '$' => [0x10, 0x3C, 0x50, 0x38, 0x14, 0x78, 0x10, 0x00],
        '%' => [0x60, 0x64, 0x08, 0x10, 0x20, 0x4C, 0x0C, 0x00],
        '&' => [0x30, 0x48, 0x30, 0x50, 0x4C, 0x44, 0x3C, 0x00],
        '\'' => [0x10, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '(' => [0x08, 0x10, 0x20, 0x20, 0x20, 0x10, 0x08, 0x00],
        ')' => [0x20, 0x10, 0x08, 0x08, 0x08, 0x10, 0x20, 0x00],
        '*' => [0x00, 0x28, 0x10, 0x7C, 0x10, 0x28, 0x00, 0x00],
        '+' => [0x00, 0x10, 0x10, 0x7C, 0x10, 0x10, 0x00, 0x00],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x10, 0x20],
        '-' => [0x00, 0x00, 0x00, 0x7C, 0x00, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x10, 0x00],
        '/' => [0x04, 0x08, 0x08, 0x10, 0x20, 0x20, 0x40, 0x00],
        '0' => [0x38, 0x44, 0x4C, 0x54, 0x64, 0x44, 0x38, 0x00],
        '1' => [0x10, 0x30, 0x10, 0x10, 0x10, 0x10, 0x38, 0x00],
        '2' => [0x38, 0x44, 0x04, 0x08, 0x10, 0x20, 0x7C, 0x00],
        '3' => [0x38, 0x44, 0x04, 0x18, 0x04, 0x44, 0x38, 0x00],
        '4' => [0x08, 0x18, 0x28, 0x48, 0x7C, 0x08, 0x08, 0x00],
        '5' => [0x7C, 0x40, 0x78, 0x04, 0x04, 0x44, 0x38, 0x00],
        '6' => [0x18, 0x20, 0x40, 0x78, 0x44, 0x44, 0x38, 0x00],
        '7' => [0x7C, 0x04, 0x08, 0x10, 0x20, 0x20, 0x20, 0x00],
        '8' => [0x38, 0x44, 0x44, 0x38, 0x44, 0x44, 0x38, 0x00],
        '9' => [0x38, 0x44, 0x44, 0x3C, 0x04, 0x08, 0x30, 0x00],
        ':' => [0x00, 0x00, 0x10, 0x00, 0x00, 0x10, 0x00, 0x00],
        ';' => [0x00, 0x00, 0x10, 0x00, 0x00, 0x10, 0x10, 0x20],
        '<' => [0x04, 0x08, 0x10, 0x20, 0x10, 0x08, 0x04, 0x00],
        '=' => [0x00, 0x00, 0x7C, 0x00, 0x7C, 0x00, 0x00, 0x00],
        '>' => [0x20, 0x10, 0x08, 0x04, 0x08, 0x10, 0x20, 0x00],
        '?' => [0x38, 0x44, 0x04, 0x08, 0x10, 0x00, 0x10, 0x00],
        '@' => [0x38, 0x44, 0x5C, 0x54, 0x5C, 0x40, 0x3C, 0x00],
        'A' => [0x38, 0x44, 0x44, 0x7C, 0x44, 0x44, 0x44, 0x00],
        'B' => [0x78, 0x44, 0x44, 0x78, 0x44, 0x44, 0x78, 0x00],
        'C' => [0x38, 0x44, 0x40, 0x40, 0x40, 0x44, 0x38, 0x00],
        'D' => [0x78, 0x44, 0x44, 0x44, 0x44, 0x44, 0x78, 0x00],
        'E' => [0x7C, 0x40, 0x40, 0x78, 0x40, 0x40, 0x7C, 0x00],
        'F' => [0x7C, 0x40, 0x40, 0x78, 0x40, 0x40, 0x40, 0x00],
        'G' => [0x38, 0x44, 0x40, 0x5C, 0x44, 0x44, 0x3C, 0x00],
        'H' => [0x44, 0x44, 0x44, 0x7C, 0x44, 0x44, 0x44, 0x00],
        'I' => [0x38, 0x10, 0x10, 0x10, 0x10, 0x10, 0x38, 0x00],
        'J' => [0x1C, 0x08, 0x08, 0x08, 0x08, 0x48, 0x30, 0x00],
        'K' => [0x44, 0x48, 0x50, 0x60, 0x50, 0x48, 0x44, 0x00],
        'L' => [0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7C, 0x00],
        'M' => [0x44, 0x6C, 0x54, 0x54, 0x44, 0x44, 0x44, 0x00],
        'N' => [0x44, 0x64, 0x54, 0x4C, 0x44, 0x44, 0x44, 0x00],
        'O' => [0x38, 0x44, 0x44, 0x44, 0x44, 0x44, 0x38, 0x00],
        'P' => [0x78, 0x44, 0x44, 0x78, 0x40, 0x40, 0x40, 0x00],
        'Q' => [0x38, 0x44, 0x44, 0x44, 0x54, 0x48, 0x34, 0x00],
        'R' => [0x78, 0x44, 0x44, 0x78, 0x50, 0x48, 0x44, 0x00],
        'S' => [0x38, 0x44, 0x40, 0x38, 0x04, 0x44, 0x38, 0x00],
        'T' => [0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00],
        'U' => [0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x38, 0x00],
        'V' => [0x44, 0x44, 0x44, 0x44, 0x44, 0x28, 0x10, 0x00],
        'W' => [0x44, 0x44, 0x44, 0x54, 0x54, 0x6C, 0x44, 0x00],
        'X' => [0x44, 0x44, 0x28, 0x10, 0x28, 0x44, 0x44, 0x00],
        'Y' => [0x44, 0x44, 0x28, 0x10, 0x10, 0x10, 0x10, 0x00],
        'Z' => [0x7C, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7C, 0x00],
        '[' => [0x38, 0x20, 0x20, 0x20, 0x20, 0x20, 0x38, 0x00],
        '\\' => [0x40, 0x20, 0x20, 0x10, 0x08, 0x08, 0x04, 0x00],
        ']' => [0x38, 0x08, 0x08, 0x08, 0x08, 0x08, 0x38, 0x00],
        '^' => [0x10, 0x28, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0x00],
        '`' => [0x20, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        'a' => [0x00, 0x00, 0x38, 0x04, 0x3C, 0x44, 0x3C, 0x00],
        'b' => [0x40, 0x40, 0x78, 0x44, 0x44, 0x44, 0x78, 0x00],
        'c' => [0x00, 0x00, 0x38, 0x44, 0x40, 0x44, 0x38, 0x00],
        'd' => [0x04, 0x04, 0x3C, 0x44, 0x44, 0x44, 0x3C, 0x00],
        'e' => [0x00, 0x00, 0x38, 0x44, 0x7C, 0x40, 0x38, 0x00],
        'f' => [0x18, 0x24, 0x20, 0x78, 0x20, 0x20, 0x20, 0x00],
        'g' => [0x00, 0x00, 0x3C, 0x44, 0x44, 0x3C, 0x04, 0x38],
        'h' => [0x40, 0x40, 0x78, 0x44, 0x44, 0x44, 0x44, 0x00],
        'i' => [0x10, 0x00, 0x30, 0x10, 0x10, 0x10, 0x38, 0x00],
        'j' => [0x08, 0x00, 0x18, 0x08, 0x08, 0x08, 0x48, 0x30],
        'k' => [0x40, 0x40, 0x44, 0x48, 0x70, 0x48, 0x44, 0x00],
        'l' => [0x30, 0x10, 0x10, 0x10, 0x10, 0x10, 0x38, 0x00],
        'm' => [0x00, 0x00, 0x68, 0x54, 0x54, 0x44, 0x44, 0x00],
        'n' => [0x00, 0x00, 0x78, 0x44, 0x44, 0x44, 0x44, 0x00],
        'o' => [0x00, 0x00, 0x38, 0x44, 0x44, 0x44, 0x38, 0x00],
        'p' => [0x00, 0x00, 0x78, 0x44, 0x44, 0x78, 0x40, 0x40],
        'q' => [0x00, 0x00, 0x3C, 0x44, 0x44, 0x3C, 0x04, 0x04],
        'r' => [0x00, 0x00, 0x58, 0x64, 0x40, 0x40, 0x40, 0x00],
        's' => [0x00, 0x00, 0x3C, 0x40, 0x38, 0x04, 0x78, 0x00],
        't' => [0x20, 0x20, 0x78, 0x20, 0x20, 0x24, 0x18, 0x00],
        'u' => [0x00, 0x00, 0x44, 0x44, 0x44, 0x4C, 0x34, 0x00],
        'v' => [0x00, 0x00, 0x44, 0x44, 0x44, 0x28, 0x10, 0x00],
        'w' => [0x00, 0x00, 0x44, 0x44, 0x54, 0x54, 0x28, 0x00],
        'x' => [0x00, 0x00, 0x44, 0x28, 0x10, 0x28, 0x44, 0x00],
        'y' => [0x00, 0x00, 0x44, 0x44, 0x44, 0x3C, 0x04, 0x38],
        'z' => [0x00, 0x00, 0x7C, 0x08, 0x10, 0x20, 0x7C, 0x00],
        '{' => [0x0C, 0x10, 0x10, 0x20, 0x10, 0x10, 0x0C, 0x00],
        '|' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00],
        '}' => [0x30, 0x08, 0x08, 0x04, 0x08, 0x08, 0x30, 0x00],
        '~' => [0x00, 0x00, 0x24, 0x54, 0x48, 0x00, 0x00, 0x00],
        _ => [0x7C, 0x7C, 0x7C, 0x7C, 0x7C, 0x7C, 0x7C, 0x00], // Block for unknown chars
    }
}
