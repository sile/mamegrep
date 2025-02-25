use std::{num::NonZeroUsize, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orfail::OrFail;
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{AppState, Focus},
    canvas::{Canvas, Token, TokenStyle},
    git::{ContextLines, Line},
};

#[derive(Debug, Default)]
pub struct SearchResultWidget {}

impl SearchResultWidget {
    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        if let Some(error) = &state.search_result.error {
            self.render_error(state, canvas, error);
            return;
        }

        self.render_header_line(state, canvas);
        self.render_files(state, canvas);
    }

    fn render_error(&self, state: &AppState, canvas: &mut Canvas, error: &str) {
        let style = if state.focus.is_editing() {
            TokenStyle::Plain
        } else {
            TokenStyle::Bold
        };

        canvas.drawln(Token::with_style("[RESULT]: error", style));
        canvas.drawln(Token::new(error));
    }

    fn render_header_line(&self, state: &AppState, canvas: &mut Canvas) {
        let style = if state.focus.is_editing() {
            TokenStyle::Plain
        } else {
            TokenStyle::Bold
        };

        canvas.drawln(Token::with_style(
            format!(
                "[RESULT]: {} hits, {} lines, {} files",
                state.search_result.hit_texts(),
                state.search_result.hit_lines(),
                state.search_result.hit_files()
            ),
            style,
        ));
    }

    fn render_files(&self, state: &AppState, canvas: &mut Canvas) {
        for (file_index, (file, lines)) in state.search_result.files.iter().enumerate() {
            if canvas.is_frame_exceeded() {
                break;
            }

            state.cursor.render_for_file(canvas, file);
            canvas.draw(Token::new(format!("{}# ", file_index + 1)));
            canvas.draw(Token::with_style(
                format!("{}", file.display()),
                TokenStyle::Underlined,
            ));
            canvas.draw(Token::new(format!(
                " ({} hits, {} lines)",
                state.search_result.hit_texts_in_file(file),
                state.search_result.hit_lines_in_file(file)
            )));

            if state.collapsed.contains(file) {
                canvas.drawln(Token::new("…"));
            } else {
                canvas.newline();
                self.render_lines(state, canvas, file, lines);
            }
        }
    }

    fn render_lines(&self, state: &AppState, canvas: &mut Canvas, file: &PathBuf, lines: &[Line]) {
        for line in lines.iter().filter(|l| l.hit) {
            if canvas.is_frame_exceeded() {
                break;
            }

            let focused = state.cursor.is_line_focused(file, line.number);
            if focused {
                self.render_before_lines(state, canvas, lines, line);
            }
            self.render_line(state, canvas, file, line);
            if focused {
                self.render_after_lines(state, canvas, lines, line);
            }
        }
    }

    fn render_line(&self, state: &AppState, canvas: &mut Canvas, file: &PathBuf, line: &Line) {
        state.cursor.render_for_line(canvas, file, line.number);

        canvas.draw(Token::new(format!(
            "[{:>width$}] ",
            line.number,
            width = state.search_result.max_line_width
        )));
        let col_offset = canvas.cursor().col;
        canvas.draw(Token::new(&line.text));
        self.highlight_line(state, canvas, file, line, col_offset);
        canvas.newline();
    }

    fn highlight_line(
        &self,
        state: &AppState,
        canvas: &mut Canvas,
        file: &PathBuf,
        line: &Line,
        mut col_offset: usize,
    ) {
        let hit_texts = state.search_result.hit_texts_in_line(file, line.number);
        let mut line_text = &line.text[..];
        for hit_text in hit_texts {
            let Some(i) = line_text.find(hit_text) else {
                // Normally, this branch should not be executed.
                // (Possibly, the file was edited during the git grep call.)
                continue;
            };

            col_offset += line_text[..i].width();
            canvas.set_cursor_col(col_offset);
            canvas.draw(Token::with_style(hit_text, TokenStyle::Reverse));

            col_offset += hit_text.width();
            line_text = &line_text[i + hit_text.len()..];
        }
    }

    fn render_before_lines(
        &self,
        state: &AppState,
        canvas: &mut Canvas,
        lines: &[Line],
        current_line: &Line,
    ) {
        if state.grep.context_lines == ContextLines::MIN {
            return;
        }

        canvas.newline();
        let end = lines
            .binary_search_by_key(&current_line.number, |l| l.number)
            .expect("infallible");
        let start = end.saturating_sub(state.grep.context_lines.0);
        for line in &lines[start..end] {
            canvas.drawln(Token::new(format!(
                "      {:>width$}| {}",
                "",
                line.text,
                width = state.search_result.max_line_width,
            )));
        }
    }

    fn render_after_lines(
        &self,
        state: &AppState,
        canvas: &mut Canvas,
        lines: &[Line],
        current_line: &Line,
    ) {
        if state.grep.context_lines == ContextLines::MIN {
            return;
        }

        let start = lines
            .binary_search_by_key(&current_line.number, |l| l.number)
            .expect("infallible")
            + 1;
        let end = (start + state.grep.context_lines.0).min(lines.len());
        for line in &lines[start..end] {
            canvas.drawln(Token::new(format!(
                "      {:>width$}| {}",
                "",
                line.text,
                width = state.search_result.max_line_width,
            )));
        }
        canvas.newline();
    }

    pub fn handle_key_event(
        &mut self,
        state: &mut AppState,
        event: KeyEvent,
    ) -> orfail::Result<()> {
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            match event.code {
                KeyCode::Char('p') => state.cursor_up(),
                KeyCode::Char('n') => state.cursor_down(),
                KeyCode::Char('f') => state.cursor_right(),
                KeyCode::Char('b') => state.cursor_left(),
                _ => {}
            }
            return Ok(());
        }

        match event.code {
            // TODO: recenter
            KeyCode::Char('/' | 'e') => state.set_focus(Focus::Pattern),
            KeyCode::Char('a') => state.set_focus(Focus::AndPattern),
            KeyCode::Char('n') => state.set_focus(Focus::NotPattern),
            KeyCode::Char('r') => state.set_focus(Focus::Revision),
            KeyCode::Char('p') => state.set_focus(Focus::Path),
            KeyCode::Char('i') => state.flip_grep_flag(|f| &mut f.ignore_case).or_fail()?,
            KeyCode::Char('u') => state.flip_grep_flag(|f| &mut f.untracked).or_fail()?,
            KeyCode::Char('I') => state.flip_grep_flag(|f| &mut f.no_index).or_fail()?,
            KeyCode::Char('R') => state.flip_grep_flag(|f| &mut f.no_recursive).or_fail()?,
            KeyCode::Char('w') => state.flip_grep_flag(|f| &mut f.word_regexp).or_fail()?,
            KeyCode::Char('F') if !(state.grep.perl_regexp || state.grep.extended_regexp) => {
                state.flip_grep_flag(|f| &mut f.fixed_strings).or_fail()?;
            }
            KeyCode::Char('E') if !(state.grep.fixed_strings || state.grep.perl_regexp) => {
                state.flip_grep_flag(|f| &mut f.extended_regexp).or_fail()?;
            }
            KeyCode::Char('P') if !(state.grep.fixed_strings || state.grep.extended_regexp) => {
                state.flip_grep_flag(|f| &mut f.perl_regexp).or_fail()?;
            }
            KeyCode::Char('+')
                if state.cursor.is_line_level() && state.grep.context_lines < ContextLines::MAX =>
            {
                state.grep.context_lines.0 += 1;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('-')
                if state.cursor.is_line_level() && state.grep.context_lines > ContextLines::MIN =>
            {
                state.grep.context_lines.0 -= 1;
                state.regrep().or_fail()?;
            }
            KeyCode::Up => state.cursor_up(),
            KeyCode::Down => state.cursor_down(),
            KeyCode::Right => state.cursor_right(),
            KeyCode::Left => state.cursor_left(),
            KeyCode::Char('t') | KeyCode::Tab => {
                state.toggle_expansion();
            }
            KeyCode::Char('T') => {
                state.toggle_all_expansion();
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct Cursor {
    pub file: Option<PathBuf>,
    pub line_number: Option<NonZeroUsize>,
}

impl Cursor {
    pub fn is_file_level(&self) -> bool {
        self.file.is_some() && self.line_number.is_none()
    }

    pub fn is_line_level(&self) -> bool {
        self.line_number.is_some()
    }

    pub fn render_for_file(&self, canvas: &mut Canvas, file: &PathBuf) {
        if self.is_file_level() && self.file.as_ref() == Some(file) {
            canvas.draw(Token::new("-> "));
        } else {
            canvas.draw(Token::new("   "));
        }
    }

    pub fn render_for_line(&self, canvas: &mut Canvas, file: &PathBuf, line_number: NonZeroUsize) {
        if self.is_line_focused(file, line_number) {
            canvas.draw(Token::new("---> "));
        } else {
            canvas.draw(Token::new("     "));
        }
    }

    pub fn is_line_focused(&self, file: &PathBuf, line_number: NonZeroUsize) -> bool {
        self.file.as_ref() == Some(file) && self.line_number == Some(line_number)
    }
}
