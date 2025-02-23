use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orfail::OrFail;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

    fn available_columns(&self, _state: &AppState) -> usize {
        // TODO: use terminal size columns
        // TODO: use canvas.size().columns
        10
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
        if state.dirty {
            self.update_cursor_position(state);
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
        self.render_grep_args(&state.grep.args(state.focus), canvas, state);
        canvas.newline();
    }

    fn render_grep_args(&self, args: &[GrepArg], canvas: &mut Canvas, state: &AppState) {
        let columns = self.available_columns(state);
        for (i, arg) in args.iter().enumerate() {
            // TODO: arg group
            let width = arg.width(state.focus) + 1; // +1 for ' ' prefix
            if i > 0 && Self::COL_OFFSET + width > columns {
                canvas.newline();
                canvas.set_cursor_col(Self::COL_OFFSET);
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
    }

    fn update_cursor_position(&self, state: &mut AppState) {
        let columns = self.available_columns(state);
        let mut pos = Self::START_POSITION;
        for (i, arg) in state.grep.args(state.focus).into_iter().enumerate() {
            let focused = arg.kind.is_focused(state.focus);
            // TODO: arg group
            let width = arg.width(state.focus) + 1; // +1 for ' '
            if i > 0 && Self::COL_OFFSET + width > columns {
                pos.row += 1;
                pos.col = Self::COL_OFFSET;
            }
            if focused {
                pos.col += arg.text[0..self.index].width() + 1; // +1 for cursor
                state.show_terminal_cursor = Some(pos);
                return;
            }
            pos.col += width;
        }
    }

    pub fn handle_focus_change(&mut self, state: &mut AppState) {
        let Some(arg) = state.focused_arg_mut() else {
            return;
        };
        self.original_text = arg.text.clone();
        self.index = arg.len();
        self.update_cursor_position(state);
        state.dirty = true;
    }
}
