use orfail::OrFail;
use tuinix::{TerminalPosition, TerminalStyle};

use crate::{
    action::Action,
    app::{AppState, Focus},
    canvas::{Canvas, Token},
    git::GrepArg,
};

#[derive(Debug, Default)]
pub struct CommandEditorWidget {
    original_text: String,
    index: usize,
    available_cols: usize,
}

impl CommandEditorWidget {
    const ROW_OFFSET: usize = 1;
    const COL_OFFSET: usize = "$ git".len();

    pub fn set_available_cols(&mut self, cols: usize) {
        self.available_cols = cols;
    }

    pub fn handle_focus_change(&mut self, state: &mut AppState) {
        let Some(arg) = state.focused_arg_mut() else {
            return;
        };
        self.original_text = arg.text.clone();
        self.index = arg.len();
        state.dirty = true;
    }

    pub fn handle_action(&mut self, state: &mut AppState, action: Action) -> orfail::Result<()> {
        match action {
            Action::AcceptInput => {
                state.regrep().or_fail()?;
                state.focus = Focus::SearchResult;
                state.dirty = true;
            }
            Action::InsertChar => {
                let c = state.last_input_char;
                state.focused_arg_mut().or_fail()?.insert(self.index, c);
                self.index += c.len_utf8();
                state.dirty = true;
            }
            Action::DeleteBackward => {
                let arg = state.focused_arg_mut().or_fail()?;
                if let Some(c) = arg.prev_char(self.index) {
                    self.index -= c.len_utf8();
                    arg.remove(self.index).or_fail()?;
                    state.dirty = true;
                }
            }
            Action::DeleteChar => {
                let arg = state.focused_arg_mut().or_fail()?;
                if arg.remove(self.index).is_some() {
                    state.dirty = true;
                }
            }
            Action::MoveBackward | Action::CursorLeft => {
                let arg = state.focused_arg_mut().or_fail()?;
                if let Some(c) = arg.prev_char(self.index) {
                    self.index -= c.len_utf8();
                    state.dirty = true;
                }
            }
            Action::MoveForward | Action::CursorRight => {
                let arg = state.focused_arg_mut().or_fail()?;
                if let Some(c) = arg.next_char(self.index) {
                    self.index += c.len_utf8();
                    state.dirty = true;
                }
            }
            Action::MoveToStart => {
                if self.index > 0 {
                    self.index = 0;
                    state.dirty = true;
                }
            }
            Action::MoveToEnd => {
                let arg = state.focused_arg_mut().or_fail()?;
                if self.index < arg.len() {
                    self.index = arg.len();
                    state.dirty = true;
                }
            }
            Action::ClearArg => {
                let arg = state.focused_arg_mut().or_fail()?;
                arg.text = self.original_text.clone();
                state.regrep().or_fail()?;
                state.focus = Focus::SearchResult;
                state.dirty = true;
            }
            _ => {}
        }

        Ok(())
    }

    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        if state.focus.is_editing() {
            canvas.drawln(Token::with_style(
                "[COMMAND]: editing…",
                TerminalStyle::new().bold(),
            ));
        } else {
            canvas.drawln(Token::with_style("[COMMAND]", TerminalStyle::new()));
        }

        canvas.draw(Token::new("$ git"));

        self.render_grep_args(state, canvas, &state.grep.args(state.focus));
    }

    fn render_grep_args(&self, state: &AppState, canvas: &mut Canvas, args: &[GrepArg]) {
        let multiline = self.is_multiline(state);
        for arg in args {
            let focused = arg.kind.is_focused(state.focus);
            if multiline && arg.multiline_head {
                canvas.newline();
                canvas.set_cursor_col(Self::COL_OFFSET);
            }
            let style = if focused {
                TerminalStyle::new().bold()
            } else {
                TerminalStyle::new()
            };
            canvas.draw(Token::with_style(
                format!(" {}", arg.maybe_quoted_text(state.focus)),
                style,
            ));
        }
        canvas.newline();
    }

    pub fn update_cursor_position(&self, state: &mut AppState) {
        if !state.focus.is_editing() {
            state.show_terminal_cursor = None;
            return;
        }

        let multiline = self.is_multiline(state);
        let mut pos = TerminalPosition::row_col(Self::ROW_OFFSET, Self::COL_OFFSET);
        for arg in state.grep.args(state.focus) {
            let focused = arg.kind.is_focused(state.focus);
            if multiline && arg.multiline_head {
                pos.row += 1;
                pos.col = Self::COL_OFFSET;
            }
            pos.col += 1; // for ' ' prefix

            if focused {
                pos.col += mame::terminal::str_cols(&arg.text[0..self.index]);
                state.show_terminal_cursor = Some(pos);
                return;
            } else {
                pos.col += arg.width(state.focus);
            }
        }
    }

    fn is_multiline(&self, state: &AppState) -> bool {
        let cols = Self::COL_OFFSET
            + state
                .grep
                .args(state.focus)
                .iter()
                .map(|a| a.width(state.focus) + 1) // +1 for ' ' prefix
                .sum::<usize>();
        cols > self.available_cols
    }
}
