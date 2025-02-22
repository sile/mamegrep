use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orfail::OrFail;
use unicode_width::UnicodeWidthChar;

use crate::{
    app::{AppState, Focus},
    canvas::{Canvas, Token, TokenPosition, TokenStyle},
    git::{GrepArg, GrepArgKind},
};

#[derive(Debug, Default)]
pub struct CommandEditorWidget {
    original_text: String,
    index: usize,
}

impl CommandEditorWidget {
    fn available_columns(&self, _state: &AppState) -> usize {
        // TODO: use terminal size columns
        // TODO: use canvas.size().columns
        10
    }

    pub fn handle_focus_change(&mut self, state: &mut AppState) -> orfail::Result<()> {
        let columns = self.available_columns(state);
        let offset = 8; // TODO: const
        let mut row = 1;
        let mut col = offset;
        for arg in state.grep.args() {
            let is_head_arg = offset == col;
            let token_width = arg.width(state.focus) + 1; // +1 for ' '
            if !is_head_arg && offset + token_width > columns {
                row += 1;
                col = offset;
            }
            col += token_width;
            match (arg.kind, state.focus) {
                (GrepArgKind::Pattern, Focus::Pattern)
                | (GrepArgKind::AndPattern, Focus::AndPattern)
                | (GrepArgKind::NotPattern, Focus::NotPattern)
                | (GrepArgKind::Revision, Focus::Revision)
                | (GrepArgKind::Path, Focus::Path) => {
                    let arg = state.focused_arg_mut().or_fail()?;
                    self.original_text = arg.text.clone();
                    self.index = arg.len();
                    state.show_terminal_cursor = Some(TokenPosition::row_col(row, col));
                    break;
                }
                _ => {}
            }
        }

        state.dirty = true;
        Ok(())
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
                let arg = state.focused_arg_mut().or_fail()?;
                arg.text = self.original_text.clone();
                state.regrep().or_fail()?;
                state.focus = Focus::SearchResult;
                state.dirty = true;
                state.show_terminal_cursor = None;
            }
            (false, KeyCode::Char(c))
                if c.is_alphanumeric() || c.is_ascii_graphic() || c == ' ' =>
            {
                state.focused_arg_mut().or_fail()?.insert(self.index, c);
                self.index += c.len_utf8();
                // TODO: consider row change
                state.show_terminal_cursor.as_mut().or_fail()?.col += c.width().or_fail()?;
                state.dirty = true;
            }
            (false, KeyCode::Backspace) => {
                if self.index > 0 {
                    let arg = state.focused_arg_mut().or_fail()?;
                    let c = arg.prev_char(self.index).or_fail()?;
                    self.index -= c.len_utf8();
                    arg.remove(self.index).or_fail()?;
                    state.show_terminal_cursor.as_mut().or_fail()?.col -= c.width().or_fail()?;
                    state.dirty = true;
                }
            }
            (false, KeyCode::Delete) | (true, KeyCode::Char('d')) => {
                let arg = state.focused_arg_mut().or_fail()?;
                if arg.remove(self.index).is_some() {
                    state.dirty = true;
                }
            }
            (false, KeyCode::Left) | (true, KeyCode::Char('b')) => {
                if self.index > 0 {
                    let arg = state.focused_arg_mut().or_fail()?;
                    let c = arg.prev_char(self.index).or_fail()?;
                    self.index -= c.len_utf8();
                    state.show_terminal_cursor.as_mut().or_fail()?.col -= c.width().or_fail()?;
                    state.dirty = true;
                }
            }
            (false, KeyCode::Right) | (true, KeyCode::Char('f')) => {
                let arg = state.focused_arg_mut().or_fail()?;
                if let Some(c) = arg.next_char(self.index) {
                    self.index += c.len_utf8();
                    state.show_terminal_cursor.as_mut().or_fail()?.col += c.width().or_fail()?;
                    state.dirty = true;
                }
            }
            (true, KeyCode::Char('a')) => {
                if self.index > 0 {
                    let arg = state.focused_arg_mut().or_fail()?;
                    let width = arg.text[..self.index]
                        .chars()
                        .map(|c| c.width().unwrap_or_default())
                        .sum::<usize>();
                    self.index = 0;
                    state.show_terminal_cursor.as_mut().or_fail()?.col -= width;
                    state.dirty = true;
                }
            }
            (true, KeyCode::Char('e')) => {
                let arg = state.focused_arg_mut().or_fail()?;
                if self.index < arg.len() {
                    let width = arg.text[self.index..]
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
            canvas.draw(Token::new("-> "));
        } else {
            canvas.draw(Token::new("   "));
        }
        canvas.draw(Token::new("$ git"));
        self.render_grep_args(&state.grep.args(), canvas, state);
        canvas.newline();
    }

    fn render_grep_args(&self, args: &[GrepArg], canvas: &mut Canvas, state: &AppState) {
        let columns = self.available_columns(state);
        let offset = canvas.cursor().col;
        for arg in args {
            let is_head_arg = offset == canvas.cursor().col;
            // TODO: consider ' ' prefix
            if !is_head_arg && offset + arg.width(state.focus) > columns {
                canvas.newline();

                let mut cursor = canvas.cursor();
                cursor.col = offset;
                canvas.set_cursor(cursor);
            }
            canvas.draw(Token::new(format!(" {}", arg.text(state.focus))));
        }
    }
}
