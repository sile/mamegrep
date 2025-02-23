use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orfail::OrFail;
use unicode_width::UnicodeWidthStr;

use crate::{
    app::{AppState, Focus},
    canvas::{Canvas, Token, TokenPosition, TokenStyle},
    git::GrepArg,
};

#[derive(Debug, Default)]
pub struct CommandEditorWidget {
    original_text: String,
    index: usize,
}

impl CommandEditorWidget {
    const ROW_OFFSET: usize = 1;
    const COL_OFFSET: usize = "-> $ git ".len();
    const START_POSITION: TokenPosition =
        TokenPosition::row_col(Self::ROW_OFFSET, Self::COL_OFFSET);

    pub fn handle_focus_change(&mut self, state: &mut AppState) {
        let Some(arg) = state.focused_arg_mut() else {
            return;
        };
        self.original_text = arg.text.clone();
        self.index = arg.len();
        self.update_cursor_position(state);
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
            }
            (false, KeyCode::Char(c))
                if c.is_alphanumeric() || c.is_ascii_graphic() || c == ' ' =>
            {
                state.focused_arg_mut().or_fail()?.insert(self.index, c);
                self.index += c.len_utf8();
                state.dirty = true;
            }
            (false, KeyCode::Backspace) => {
                let arg = state.focused_arg_mut().or_fail()?;
                if let Some(c) = arg.prev_char(self.index) {
                    self.index -= c.len_utf8();
                    arg.remove(self.index).or_fail()?;
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
                let arg = state.focused_arg_mut().or_fail()?;
                if let Some(c) = arg.prev_char(self.index) {
                    self.index -= c.len_utf8();
                    state.dirty = true;
                }
            }
            (false, KeyCode::Right) | (true, KeyCode::Char('f')) => {
                let arg = state.focused_arg_mut().or_fail()?;
                if let Some(c) = arg.next_char(self.index) {
                    self.index += c.len_utf8();
                    state.dirty = true;
                }
            }
            (true, KeyCode::Char('a')) => {
                if self.index > 0 {
                    self.index = 0;
                    state.dirty = true;
                }
            }
            (true, KeyCode::Char('e')) => {
                let arg = state.focused_arg_mut().or_fail()?;
                if self.index < arg.len() {
                    self.index = state.grep.pattern.len();
                    state.dirty = true;
                }
            }
            _ => {}
        }

        if state.dirty {
            self.update_cursor_position(state);
        }

        Ok(())
    }

    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        canvas.drawln(Token::with_style("[COMMAND]", TokenStyle::Bold));

        let prefix = if state.focus.is_editing() { "->" } else { "  " };
        canvas.draw(Token::new(format!("{prefix} $ git")));

        self.render_grep_args(state, canvas, &state.grep.args(state.focus));
    }

    fn render_grep_args(&self, state: &AppState, canvas: &mut Canvas, args: &[GrepArg]) {
        let columns = self.available_columns(state);
        for arg in args {
            let width = arg.width(state.focus) + 1; // +1 for ' ' prefix
            if arg.line_breakable && Self::COL_OFFSET + width > columns {
                canvas.newline();
                canvas.set_cursor_col(Self::COL_OFFSET - 1);
            }
            let style = if arg.kind.is_focused(state.focus) {
                TokenStyle::Bold
            } else {
                TokenStyle::Plain
            };
            canvas.draw(Token::with_style(
                format!(" {}", arg.text(state.focus)),
                style,
            ));
        }
        canvas.newline();
    }

    fn update_cursor_position(&self, state: &mut AppState) {
        if !state.focus.is_editing() {
            state.show_terminal_cursor = None;
            return;
        }

        let columns = self.available_columns(state);
        let mut pos = Self::START_POSITION;
        for arg in state.grep.args(state.focus) {
            let focused = arg.kind.is_focused(state.focus);
            let width = arg.width(state.focus) + 1; // +1 for ' '
            if arg.line_breakable && Self::COL_OFFSET + width > columns {
                pos.row += 1;
                pos.col = Self::COL_OFFSET;
            }
            if focused {
                pos.col += arg.text[0..self.index].width();
                state.show_terminal_cursor = Some(pos);
                return;
            }
            pos.col += width;
        }
    }

    fn available_columns(&self, _state: &AppState) -> usize {
        // TODO: use terminal size columns
        // TODO: use canvas.size().columns
        10
    }
}
