use crate::{
    app::{AppState, Focus},
    canvas::{Canvas, Token, TokenPosition, TokenStyle},
};

#[derive(Debug, Default)]
pub struct LegendWidget {
    pub hide: bool,
}

impl LegendWidget {
    pub const COLUMNS: usize = 22;

    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        canvas.set_cursor(TokenPosition::row(0));

        let editing = !matches!(state.focus, Focus::SearchResult);
        if self.hide {
            let col = canvas.frame_size().cols - 11;
            canvas.set_col_offset(col);
            if editing {
                canvas.drawln(Token::new("+----------"));
            } else {
                canvas.drawln(Token::new("+- s(h)ow -"));
            }
            return;
        }

        if editing {
            self.render_editing_legend(state, canvas);
        } else {
            self.render_search_result_legend(state, canvas);
        }
    }

    fn render_editing_legend(&self, _state: &AppState, canvas: &mut Canvas) {
        if canvas.frame_size().cols < Self::COLUMNS {
            return;
        }
        canvas.set_col_offset(canvas.frame_size().cols - Self::COLUMNS);

        canvas.drawln(Token::with_style(
            "|[ACTIONS]            ",
            TokenStyle::Bold,
        ));
        canvas.drawln(Token::new("| quit       [ESC,C-c]"));
        canvas.drawln(Token::new("|                     "));
        canvas.drawln(Token::new("| search       [ENTER]"));
        canvas.drawln(Token::new("| preview        [TAB]"));
        canvas.drawln(Token::new("| cancel         [C-g]"));
        canvas.drawln(Token::new("|                     "));
        canvas.drawln(Token::new("| (BACKSPACE)         "));
        canvas.drawln(Token::new("| (DELETE)       [C-d]"));
        canvas.drawln(Token::new("| (←)            [C-b]"));
        canvas.drawln(Token::new("| (→)            [C-f]"));
        canvas.drawln(Token::new("| go to head     [C-a]"));
        canvas.drawln(Token::new("| go to tail     [C-e]"));
        canvas.drawln(Token::new("+---------------------"));
    }

    fn render_search_result_legend(&self, state: &AppState, canvas: &mut Canvas) {
        if canvas.frame_size().cols < Self::COLUMNS {
            return;
        }
        canvas.set_col_offset(canvas.frame_size().cols - Self::COLUMNS);

        canvas.drawln(Token::with_style(
            "|[ACTIONS]            ",
            TokenStyle::Bold,
        ));
        canvas.drawln(Token::new("| (q)uit     [ESC,C-c]"));
        canvas.drawln(Token::new("|                     "));
        canvas.drawln(Token::new("| (e)dit pattern   [/]"));
        canvas.drawln(Token::new("| edit (a)nd pattern  "));
        canvas.drawln(Token::new("| edit (n)ot pattern  "));
        canvas.drawln(Token::new("| edit (r)evision     "));
        canvas.drawln(Token::new("| edit (p)ath         "));
        canvas.drawln(Token::new("|                     "));

        // TODO: conditional
        canvas.drawln(Token::new("| (t)oggle       [TAB]"));
        canvas.drawln(Token::new("| (T)oggle all        "));
        canvas.drawln(Token::new("| (↑)            [C-p]"));
        canvas.drawln(Token::new("| (↓)            [C-n]"));
        canvas.drawln(Token::new("| (←)            [C-b]"));
        canvas.drawln(Token::new("| (→)            [C-f]"));
        canvas.drawln(Token::new("| (+|-) context lines "));

        canvas.drawln(Token::new("|                     "));
        canvas.drawln(Token::with_style(
            "|[GIT GREP FLAGS]     ",
            TokenStyle::Bold,
        ));

        if state.grep.ignore_case {
            canvas.drawln(Token::new("|o --(i)gnore-case    "));
        } else {
            canvas.drawln(Token::new("|  --(i)gnore-case    "));
        }
        if state.grep.untracked {
            canvas.drawln(Token::new("|o --(u)ntracked      "));
        } else {
            canvas.drawln(Token::new("|  --(u)ntracked      "));
        }
        if state.grep.no_index {
            canvas.drawln(Token::new("|o --no-(I)ndex       "));
        } else {
            canvas.drawln(Token::new("|  --no-(I)ndex       "));
        }
        if state.grep.no_recursive {
            canvas.drawln(Token::new("|o --no-(R)ecursive   "));
        } else {
            canvas.drawln(Token::new("|  --no-(R)ecursive   "));
        }
        if state.grep.word_regexp {
            canvas.drawln(Token::new("|o --(w)ord-regexp    "));
        } else {
            canvas.drawln(Token::new("|  --(w)ord-regexp    "));
        }
        if !(state.grep.extended_regexp || state.grep.perl_regexp) {
            if state.grep.fixed_strings {
                canvas.drawln(Token::new("|o --(F)ixed-strings  "));
            } else {
                canvas.drawln(Token::new("|  --(F)ixed-strings  "));
            }
        }
        if !(state.grep.fixed_strings || state.grep.perl_regexp) {
            if state.grep.extended_regexp {
                canvas.drawln(Token::new("|o --(E)xtended-regexp"));
            } else {
                canvas.drawln(Token::new("|  --(E)xtended-regexp"));
            }
        }
        if !(state.grep.fixed_strings || state.grep.extended_regexp) {
            if state.grep.perl_regexp {
                canvas.drawln(Token::new("|o --(P)erl-regexp    "));
            } else {
                canvas.drawln(Token::new("|  --(P)erl-regexp    "));
            }
        }

        canvas.drawln(Token::new("+-------(h)ide--------"));
    }
}
