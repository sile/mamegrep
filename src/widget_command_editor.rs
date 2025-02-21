use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orfail::OrFail;
use unicode_width::UnicodeWidthStr;

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
                    break;
                }
                _ => {}
            }
        }

        match state.focus {
            Focus::Pattern => {
                self.original_text = state.grep.pattern.clone();
                self.index = state.grep.pattern.len();
            }
            _ => {}
        }

        state.dirty = true;
    }

    pub fn handle_key_event(
        &mut self,
        state: &mut AppState,
        event: KeyEvent,
    ) -> orfail::Result<()> {
        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        match event.code {
            KeyCode::Enter => {
                state.regrep().or_fail()?;
                state.focus = Focus::SearchResult;
                state.dirty = true;
                state.show_terminal_cursor = None;
            }
            KeyCode::Tab => {
                state.regrep().or_fail()?;
                state.dirty = true;
            }
            KeyCode::Char('g') if ctrl => {
                match state.focus {
                    Focus::Pattern => {
                        state.grep.pattern = self.original_text.clone();
                    }
                    _ => {}
                }
                state.regrep().or_fail()?;
                state.focus = Focus::SearchResult;
                state.dirty = true;
            }
            KeyCode::Char(_) if ctrl => {
                // ignore
            }
            // TODO: C-a, C-e, C-b, C-f, C-k, C-d, C-h
            KeyCode::Char(c) if c.is_alphanumeric() || c == ' ' => {
                // TODO: escape
                state.grep.pattern.push(c);
                state.dirty = true;
            }
            KeyCode::Backspace => {
                state.grep.pattern.pop();
                state.dirty = true;
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
