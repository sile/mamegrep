use std::{num::NonZeroUsize, path::PathBuf};

use orfail::OrFail;
use tuinix::TerminalStyle;

use crate::{
    action::Action,
    app::AppState,
    canvas::{Canvas, Token},
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

        let mut size = canvas.frame_size();
        size.rows = size.rows.saturating_sub(canvas.cursor().row);

        let mut tmp_canvas = Canvas::new(size);
        tmp_canvas.set_auto_scroll(true);
        self.render_files(state, &mut tmp_canvas);

        for line in tmp_canvas.into_frame().into_lines() {
            canvas.draw_frame_line(line);
        }
    }

    fn render_error(&self, state: &AppState, canvas: &mut Canvas, error: &str) {
        let style = if state.focus.is_editing() {
            TerminalStyle::new()
        } else {
            TerminalStyle::new().bold()
        };

        canvas.drawln(Token::with_style("[RESULT]: error", style));
        canvas.drawln(Token::new(error));
    }

    fn render_header_line(&self, state: &AppState, canvas: &mut Canvas) {
        let style = if state.focus.is_editing() {
            TerminalStyle::new()
        } else {
            TerminalStyle::new().bold()
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

            if state.cursor.render_for_file(canvas, file) {
                self.recenter(canvas);
            }
            canvas.draw(Token::new(format!("{}# ", file_index + 1)));
            canvas.draw(Token::with_style(
                format!("{}", file.display()),
                TerminalStyle::new().underline(),
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
        if state.cursor.render_for_line(canvas, file, line.number) {
            self.recenter(canvas);
        }
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

    fn recenter(&self, canvas: &mut Canvas) {
        canvas.set_auto_scroll(false);

        let current_row = canvas.cursor().row;
        let frame_rows = canvas.frame_size().rows;
        let frame_half_rows = frame_rows / 2;
        canvas.scroll(
            current_row
                .saturating_sub(frame_half_rows)
                .min(frame_half_rows),
        );
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

            col_offset += mame::terminal::str_cols(&line_text[..i]);
            canvas.set_cursor_col(col_offset);
            canvas.draw(Token::with_style(hit_text, TerminalStyle::new().reverse()));

            col_offset += mame::terminal::str_cols(hit_text);
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

    pub fn handle_action(&mut self, state: &mut AppState, action: Action) -> orfail::Result<()> {
        match action {
            Action::CursorUp => state.cursor_up(),
            Action::CursorDown => state.cursor_down(),
            Action::CursorRight => state.cursor_right(),
            Action::CursorLeft => state.cursor_left(),
            Action::SetFocus(focus) => state.set_focus(focus),
            Action::ToggleExpansion => state.toggle_expansion(),
            Action::ToggleAllExpansion => state.toggle_all_expansion(),
            Action::FlipIgnoreCase => state.flip_grep_flag(|f| &mut f.ignore_case).or_fail()?,
            Action::FlipExtendedRegexp if !(state.grep.fixed_strings || state.grep.perl_regexp) => {
                state.flip_grep_flag(|f| &mut f.extended_regexp).or_fail()?;
            }
            Action::FlipFixedStrings if !(state.grep.perl_regexp || state.grep.extended_regexp) => {
                state.flip_grep_flag(|f| &mut f.fixed_strings).or_fail()?;
            }
            Action::FlipPerlRegexp if !(state.grep.fixed_strings || state.grep.extended_regexp) => {
                state.flip_grep_flag(|f| &mut f.perl_regexp).or_fail()?;
            }
            Action::FlipContext if state.cursor.is_line_level() => {
                if state.grep.context_lines < ContextLines::MAX {
                    state.grep.context_lines.0 += 1;
                    state.regrep().or_fail()?;
                }
            }
            Action::FlipContextBefore if state.cursor.is_line_level() => {
                if state.grep.context_lines > ContextLines::MIN {
                    state.grep.context_lines.0 -= 1;
                    state.regrep().or_fail()?;
                }
            }
            Action::FlipWholeWord => {
                state.flip_grep_flag(|f| &mut f.word_regexp).or_fail()?;
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

    pub fn render_for_file(&self, canvas: &mut Canvas, file: &PathBuf) -> bool {
        if self.is_file_level() && self.file.as_ref() == Some(file) {
            canvas.draw(Token::new("-> "));
            true
        } else {
            canvas.draw(Token::new("   "));
            false
        }
    }

    pub fn render_for_line(
        &self,
        canvas: &mut Canvas,
        file: &PathBuf,
        line_number: NonZeroUsize,
    ) -> bool {
        if self.is_line_focused(file, line_number) {
            canvas.draw(Token::new("---> "));
            true
        } else {
            canvas.draw(Token::new("     "));
            false
        }
    }

    pub fn is_line_focused(&self, file: &PathBuf, line_number: NonZeroUsize) -> bool {
        self.file.as_ref() == Some(file) && self.line_number == Some(line_number)
    }
}
