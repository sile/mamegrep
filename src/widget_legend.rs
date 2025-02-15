use crate::{
    app::AppState,
    canvas::{Canvas, Token, TokenPosition},
};

#[derive(Debug, Default)]
pub struct LegendWidget {
    pub hide: bool,
    pub editing: bool, // TODO: Move to AppState
}

impl LegendWidget {
    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        // TODO: caller's responsibility
        canvas.set_cursor(TokenPosition::row(2));

        if self.hide {
            let col = canvas.frame_size().cols - 11;
            canvas.set_col_offset(col);
            canvas.drawln(Token::new("+- s(h)ow -"));
            return;
        }

        if self.editing {
            // TODO
            return;
        } else {
            self.render_main_legend(state, canvas);
        }
    }

    fn render_main_legend(&self, state: &AppState, canvas: &mut Canvas) {
        let width = 22;
        if canvas.frame_size().cols < width {
            return;
        }

        canvas.set_col_offset(canvas.frame_size().cols - width);

        canvas.drawln(Token::new("|= actions ==========="));
        canvas.drawln(Token::new("| (q)uit     [ESC,C-c]"));

        // TODO: conditional
        canvas.drawln(Token::new("| (t)oggle       [TAB]"));
        canvas.drawln(Token::new("| (T)oggle all        "));
        canvas.drawln(Token::new("| (↑)            [C-p]"));
        canvas.drawln(Token::new("| (↓)            [C-n]"));
        canvas.drawln(Token::new("| (←)            [C-b]"));
        canvas.drawln(Token::new("| (→)            [C-f]"));
        canvas.drawln(Token::new("| (+|-) context lines "));

        canvas.drawln(Token::new("|                     "));
        canvas.drawln(Token::new("|= git grep patterns ="));
        canvas.drawln(Token::new("| (e)dit pattern   [/]"));
        canvas.drawln(Token::new("| edit (a)nd pattern  "));
        canvas.drawln(Token::new("| edit (n)ot pattern  "));
        canvas.drawln(Token::new("| edit (r)evision     "));
        canvas.drawln(Token::new("| edit (p)ath         "));

        canvas.drawln(Token::new("|                     "));
        canvas.drawln(Token::new("|= git grep flags ===="));

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
