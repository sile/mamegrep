use std::{collections::VecDeque, fmt::Write, num::NonZeroUsize};

use tuinix::{EstimateCharWidth, TerminalFrame, TerminalPosition, TerminalSize, TerminalStyle};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Default)]
pub struct UnicodeCharWidthEstimator;

impl EstimateCharWidth for UnicodeCharWidthEstimator {
    fn estimate_char_width(&self, c: char) -> usize {
        c.width().unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct Canvas {
    frame: Frame,
    cursor: TerminalPosition,
    col_offset: usize,
    row_offset: usize,
    auto_scroll: bool,
}

impl Canvas {
    pub fn new(frame_size: TerminalSize) -> Self {
        Self {
            frame: Frame::new(frame_size),
            cursor: TerminalPosition::ZERO,
            col_offset: 0,
            row_offset: 0,
            auto_scroll: false,
        }
    }

    pub fn frame_size(&self) -> TerminalSize {
        self.frame.size
    }

    pub fn is_frame_exceeded(&self) -> bool {
        if self.auto_scroll {
            false
        } else {
            self.cursor.row.saturating_sub(self.row_offset) >= self.frame.size.rows
        }
    }

    pub fn cursor(&self) -> TerminalPosition {
        self.cursor
    }

    pub fn set_cursor(&mut self, position: TerminalPosition) {
        self.cursor = position;
    }

    pub fn set_cursor_col(&mut self, col: usize) {
        self.cursor.col = col;
    }

    pub fn set_col_offset(&mut self, offset: usize) {
        self.col_offset = offset;
    }

    pub fn draw(&mut self, token: Token) {
        let cols = token.cols();
        self.draw_at(self.cursor, token);
        self.cursor.col += cols;
    }

    pub fn drawln(&mut self, token: Token) {
        self.draw(token);
        self.newline();
    }

    pub fn newline(&mut self) {
        self.cursor.row += 1;
        self.cursor.col = 0;
    }

    pub fn draw_at(&mut self, mut position: TerminalPosition, token: Token) {
        if position.row < self.row_offset {
            return;
        }

        if let Some(n) = (position.row - self.row_offset).checked_sub(self.frame.size.rows) {
            if self.auto_scroll {
                self.scroll(n + 1);
            } else {
                return;
            }
        }

        position.col += self.col_offset;

        let i = position.row - self.row_offset;
        let line = &mut self.frame.lines[i];
        line.draw_token(position.col, token);
        line.split_off(self.frame.size.cols);
    }

    pub fn draw_frame_line(&mut self, line: FrameLine) {
        if self.cursor.row < self.frame.lines.len() {
            self.frame.lines[self.cursor.row] = line;
            self.cursor.row += 1;
        }
    }

    pub fn into_frame(self) -> Frame {
        self.frame
    }

    pub fn set_auto_scroll(&mut self, auto: bool) {
        self.auto_scroll = auto;
    }

    pub fn scroll(&mut self, n: usize) {
        for _ in 0..n {
            self.frame.lines.pop_front();
            self.frame.lines.push_back(FrameLine::new());
            self.row_offset += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    size: TerminalSize,
    lines: VecDeque<FrameLine>,
}

impl Frame {
    pub fn new(size: TerminalSize) -> Self {
        Self {
            size,
            lines: vec![FrameLine::new(); size.rows].into(),
        }
    }

    pub fn into_lines(self) -> impl Iterator<Item = FrameLine> {
        self.lines.into_iter()
    }

    pub fn into_terminal_frame(self) -> TerminalFrame<UnicodeCharWidthEstimator> {
        let mut frame =
            TerminalFrame::with_char_width_estimator(self.size, UnicodeCharWidthEstimator);
        for line in self.into_lines() {
            for token in line.tokens {
                let _ = write!(frame, "{}{}", token.style, token.text);
            }
            let _ = writeln!(frame, "{}", TerminalStyle::RESET);
        }
        frame
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FrameLine {
    tokens: Vec<Token>,
}

impl FrameLine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    pub fn text(&self) -> String {
        self.tokens.iter().map(|t| t.text.clone()).collect()
    }

    pub fn draw_token(&mut self, col: usize, token: Token) {
        if let Some(n) = col.checked_sub(self.cols()).and_then(NonZeroUsize::new) {
            let s: String = std::iter::repeat_n(' ', n.get()).collect();
            self.tokens.push(Token::new(s));
        }

        let mut suffix = self.split_off(col);
        let suffix = suffix.split_off(token.cols());
        self.tokens.push(token);
        self.tokens.extend(suffix.tokens);
    }

    fn split_off(&mut self, col: usize) -> Self {
        let mut acc_cols = 0;
        for i in 0..self.tokens.len() {
            if acc_cols == col {
                let suffix = self.tokens.split_off(i);
                return Self { tokens: suffix };
            }

            let token_cols = self.tokens[i].cols();
            acc_cols += token_cols;
            if acc_cols == col {
                continue;
            } else if let Some(n) = acc_cols.checked_sub(col) {
                let mut suffix = self.tokens.split_off(i);
                let token_prefix_cols = token_cols - n;
                let token_prefix = suffix[0].split_prefix_off(token_prefix_cols);
                self.tokens.push(token_prefix);
                return Self { tokens: suffix };
            }
        }

        // `col` is out of range, so no splitting is needed.
        Self::new()
    }

    pub fn cols(&self) -> usize {
        self.tokens.iter().map(|t| t.cols()).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    text: String,
    style: TerminalStyle,
}

impl Token {
    pub fn new(text: impl Into<String>) -> Self {
        Self::with_style(text, TerminalStyle::new())
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn style(&self) -> TerminalStyle {
        self.style
    }

    pub fn with_style(text: impl Into<String>, style: TerminalStyle) -> Self {
        let mut text = text.into();
        if text.chars().any(|c| c.is_control()) {
            let mut escaped_text = String::new();
            for c in text.chars() {
                if c.is_control() {
                    escaped_text.extend(c.escape_default());
                } else {
                    escaped_text.push(c);
                }
            }
            text = escaped_text;
        }
        Self { text, style }
    }

    pub fn split_prefix_off(&mut self, col: usize) -> Self {
        let mut acc_cols = 0;
        for (i, c) in self.text.char_indices() {
            if acc_cols == col {
                let suffix = self.text.split_off(i);
                return std::mem::replace(self, Self::with_style(suffix, self.style));
            }

            let next_acc_cols = acc_cols + c.width().expect("infallible");
            if next_acc_cols > col {
                // Not a char boundary.
                let suffix = self.text.split_off(i + c.len_utf8());
                let suffix = Self::with_style(suffix, self.style);
                let _ = self.text.pop();
                for _ in acc_cols..col {
                    self.text.push('…');
                }
                return std::mem::replace(self, suffix);
            }
            acc_cols = next_acc_cols;
        }

        std::mem::replace(self, Self::with_style(String::new(), self.style))
    }

    pub fn cols(&self) -> usize {
        self.text.width()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_line() -> orfail::Result<()> {
        let mut line = FrameLine::new();

        line.draw_token(2, Token::new("foo"));
        assert_eq!(line.text(), "  foo");

        line.draw_token(4, Token::new("bar"));
        assert_eq!(line.text(), "  fobar");

        line.draw_token(7, Token::new("baz"));
        assert_eq!(line.text(), "  fobarbaz");

        line.draw_token(6, Token::new("qux"));
        assert_eq!(line.text(), "  fobaquxz");

        // Control chars are escaped.
        line.draw_token(0, Token::new("0\n1"));
        assert_eq!(line.text(), "0\\n1baquxz");

        Ok(())
    }
}
