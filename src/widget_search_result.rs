use std::{num::NonZeroUsize, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    app::{AppState, Focus},
    canvas::{Canvas, Token, TokenStyle},
};

#[derive(Debug, Default)]
pub struct SearchResultWidget {
    //
}

impl SearchResultWidget {
    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
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

    pub fn handle_key_event(
        &mut self,
        state: &mut AppState,
        event: KeyEvent,
    ) -> orfail::Result<()> {
        match event.code {
            KeyCode::Char('/' | 'e') => {
                state.focus = Focus::Pattern;
                state.dirty = true;
            }
            // KeyCode::Char('a') if !editing => {
            //     state.focus = Focus::AndPattern;
            //     state.dirty = true;
            // }
            // KeyCode::Char('n') if !editing => {
            //     state.focus = Focus::NotPattern;
            //     state.dirty = true;
            // }
            // KeyCode::Char('r') if !editing => {
            //     state.focus = Focus::Revision;
            //     state.dirty = true;
            // }
            // KeyCode::Char('p') if !editing => {
            //     state.focus = Focus::Path;
            //     state.dirty = true;
            // }
            // KeyCode::Char('i') => {
            //     state.grep.ignore_case = !state.grep.ignore_case;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('u') => {
            //     state.grep.untracked = !state.grep.untracked;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('I') => {
            //     state.grep.no_index = !state.grep.no_index;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('R') => {
            //     state.grep.no_recursive = !state.grep.no_recursive;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('w') => {
            //     state.grep.word_regexp = !state.grep.word_regexp;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('F') if !(state.grep.perl_regexp || state.grep.extended_regexp) => {
            //     state.grep.fixed_strings = !state.grep.fixed_strings;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('E') if !(state.grep.fixed_strings || state.grep.perl_regexp) => {
            //     state.grep.extended_regexp = !state.grep.extended_regexp;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('P') if !(state.grep.fixed_strings || state.grep.extended_regexp) => {
            //     state.grep.perl_regexp = !state.grep.perl_regexp;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('+') if state.grep.context_lines.0 < 99 => {
            //     state.grep.context_lines.0 += 1;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Char('-') if state.grep.context_lines.0 > 1 => {
            //     state.grep.context_lines.0 -= 1;
            //     state.regrep().or_fail()?;
            // }
            // KeyCode::Up => {
            //     state.cursor_up();
            // }
            // KeyCode::Down => {
            //     state.cursor_down();
            // }
            // KeyCode::Right => {
            //     state.cursor_right();
            // }
            // KeyCode::Left => {
            //     state.cursor_left();
            // }
            // KeyCode::Char('t') => {
            //     state.toggle_expansion();
            // }
            // KeyCode::Char('T') => {
            //     state.toggle_all_expansion();
            // }
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
        if self.line_number.is_some() {
            canvas.draw(Token::new("   "));
        } else if self.file.as_ref() == Some(file) {
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
