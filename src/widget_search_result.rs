use std::{num::NonZeroUsize, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orfail::OrFail;

use crate::{
    app::{AppState, Focus},
    canvas::{Canvas, Token, TokenStyle},
    git::{ContextLines, MatchLine},
};

#[derive(Debug, Default)]
pub struct SearchResultWidget {}

impl SearchResultWidget {
    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        self.render_header_line(state, canvas);
        self.render_files(state, canvas);
    }

    fn render_header_line(&self, state: &AppState, canvas: &mut Canvas) {
        let style = if state.focus.is_editing() {
            TokenStyle::Plain
        } else {
            TokenStyle::Bold
        };
        canvas.drawln(Token::with_style(
            format!(
                "[RESULT]: {} lines, {} files",
                state.search_result.hit_lines(),
                state.search_result.hit_files()
            ),
            style,
        ));
    }

    fn render_files(&self, state: &AppState, canvas: &mut Canvas) {
        for (file, lines) in &state.search_result.files {
            state.cursor.render_for_file(canvas, file, state.focus);
            canvas.draw(Token::with_style(
                format!("{}", file.display()),
                TokenStyle::Underlined,
            ));
            canvas.draw(Token::new(format!(
                " ({} hits, {} lines)",
                state.search_result.hit_strings_in_file(file),
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

    fn render_lines(
        &self,
        state: &AppState,
        canvas: &mut Canvas,
        file: &PathBuf,
        lines: &[MatchLine],
    ) {
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
            KeyCode::Char('+') if state.grep.context_lines < ContextLines::MAX => {
                state.grep.context_lines.0 += 1;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('-') if state.grep.context_lines > ContextLines::MIN => {
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

    pub fn render_for_file(&self, canvas: &mut Canvas, file: &PathBuf, focus: Focus) {
        if !focus.is_editing() && self.is_file_level() && self.file.as_ref() == Some(file) {
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
