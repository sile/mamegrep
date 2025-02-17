use unicode_width::UnicodeWidthStr;

use crate::{
    app::AppState,
    canvas::{Canvas, Token, TokenPosition, TokenStyle},
};

#[derive(Debug, Default)]
pub struct CommandEditorWidget {}

impl CommandEditorWidget {
    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        if state.editing {
            // TODO: consider multi line
            canvas.draw(Token::with_style("-> ", TokenStyle::Bold));
        } else {
            canvas.draw(Token::new("   "));
        }
        canvas.draw(Token::new("$ git"));
        self.render_grep_args(&state.grep.args(), canvas);
        canvas.newline();
    }

    fn render_grep_args(&self, args: &[String], canvas: &mut Canvas) {
        // TODO: use canvas.size().columns
        let columns = 20;
        let offset = canvas.cursor().col;
        for arg in args {
            let is_head_arg = offset == canvas.cursor().col;
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
