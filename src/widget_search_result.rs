use std::{num::NonZeroUsize, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use orfail::OrFail;

use crate::{
    app::{AppState, Focus},
    canvas::{Canvas, Token, TokenStyle},
    git::ContextLines,
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
