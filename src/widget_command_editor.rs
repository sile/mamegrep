use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orfail::OrFail;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    app::{AppState, Focus},
    canvas::{Canvas, Token, TokenPosition, TokenStyle},
    git::GrepArgKind,
};

#[derive(Debug, Default)]
pub struct CommandEditorWidget {
    original_text: String,
    index: usize,
}

impl CommandEditorWidget {
    pub fn handle_focus_change(&mut self, state: &mut AppState) {
        // TODO: use terminal size columns
        let columns = 20;
        let offset = 8; // TODO: const
        let mut row = 1;
        let mut col = offset;
        for (kind, arg) in state.grep.args() {
            let is_head_arg = offset == col;
            let token_width = format!(" {arg}").width();
            if !is_head_arg && offset + token_width > columns {
                row += 1;
                col = offset;
            }
            col += token_width;
            match (kind, state.focus) {
                (GrepArgKind::Pattern, Focus::Pattern) => {
                    state.show_terminal_cursor = Some(TokenPosition::row_col(row, col));
                    self.original_text = state.grep.pattern.clone();
                    self.index = state.grep.pattern.len();
                    break;
                }
                _ => {}
            }
        }

        state.dirty = true;
    }

    pub fn handle_key_event(
        &mut self,
        state: &mut AppState,
        event: KeyEvent,
    ) -> orfail::Result<()> {
        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        match (ctrl, event.code) {
            (_, KeyCode::Enter) => {
                state.regrep().or_fail()?;
                state.focus = Focus::SearchResult;
                state.dirty = true;
                state.show_terminal_cursor = None;
            }
            (_, KeyCode::Tab) => {
                state.regrep().or_fail()?;
                state.dirty = true;
            }
            (true, KeyCode::Char('g')) => {
                match state.focus {
                    Focus::Pattern => {
                        state.grep.pattern = self.original_text.clone();
                    }
                    _ => {}
                }
                state.regrep().or_fail()?;
                state.focus = Focus::SearchResult;
                state.dirty = true;
                state.show_terminal_cursor = None;
            }
            (false, KeyCode::Char(c))
                if c.is_alphanumeric() || c.is_ascii_graphic() || c == ' ' =>
            {
                state.grep.pattern.insert(self.index, c);
                self.index += c.len_utf8();
                state.show_terminal_cursor.as_mut().or_fail()?.col += c.width().or_fail()?;
                state.dirty = true;
            }
            (false, KeyCode::Backspace) => {
                if self.index > 0 {
                    let c = state.grep.pattern[..self.index]
                        .chars()
                        .rev()
                        .next()
                        .or_fail()?;
                    self.index -= c.len_utf8();
                    state.grep.pattern.remove(self.index);
                    state.show_terminal_cursor.as_mut().or_fail()?.col -= c.width().or_fail()?;
                    state.dirty = true;
                }
            }
            (false, KeyCode::Delete) | (true, KeyCode::Char('d')) => {
                if self.index < state.grep.pattern.len() {
                    state.grep.pattern.remove(self.index);
                    state.dirty = true;
                }
            }
            (false, KeyCode::Left) | (true, KeyCode::Char('b')) => {
                if self.index > 0 {
                    let c = state.grep.pattern[..self.index]
                        .chars()
                        .rev()
                        .next()
                        .or_fail()?;
                    self.index -= c.len_utf8();
                    state.show_terminal_cursor.as_mut().or_fail()?.col -= c.width().or_fail()?;
                    state.dirty = true;
                }
            }
            (false, KeyCode::Right) | (true, KeyCode::Char('f')) => {
                if self.index < state.grep.pattern.len() {
                    let c = state.grep.pattern[self.index..].chars().next().or_fail()?;
                    self.index += c.len_utf8();
                    state.show_terminal_cursor.as_mut().or_fail()?.col += c.width().or_fail()?;
                    state.dirty = true;
                }
            }
            (true, KeyCode::Char('a')) => {
                if self.index > 0 {
                    let width = state.grep.pattern[..self.index]
                        .chars()
                        .map(|c| c.width().unwrap_or_default())
                        .sum::<usize>();
                    self.index = 0;
                    state.show_terminal_cursor.as_mut().or_fail()?.col -= width;
                    state.dirty = true;
                }
            }
            (true, KeyCode::Char('e')) => {
                if self.index < state.grep.pattern.len() {
                    let width = state.grep.pattern[self.index..]
                        .chars()
                        .map(|c| c.width().unwrap_or_default())
                        .sum::<usize>();
                    self.index = state.grep.pattern.len();
                    state.show_terminal_cursor.as_mut().or_fail()?.col += width;
                    state.dirty = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        canvas.drawln(Token::with_style("[COMMAND]", TokenStyle::Bold));

        if state.focus != Focus::SearchResult {
            // TODO: consider multi line
            // TODO: consider focus
            canvas.draw(Token::with_style("-> ", TokenStyle::Bold));
        } else {
            canvas.draw(Token::new("   "));
        }
        canvas.draw(Token::new("$ git"));
        self.render_grep_args(&state.grep.args(), canvas);
        canvas.newline();
    }

    fn render_grep_args(&self, args: &[(GrepArgKind, String)], canvas: &mut Canvas) {
        // TODO: use canvas.size().columns
        let columns = 20;
        let offset = canvas.cursor().col;
        for (_, arg) in args {
            let is_head_arg = offset == canvas.cursor().col;
            // TODO: consider ' ' prefix
            if !is_head_arg && offset + arg.width() > columns {
                canvas.newline();

                let mut cursor = canvas.cursor();
                cursor.col = offset;
                canvas.set_cursor(cursor);
            }
            canvas.draw(Token::new(format!(" {arg}")));
        }
    }
}
