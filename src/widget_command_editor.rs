use crate::{
    app::AppState,
    canvas::{Canvas, Token, TokenStyle},
};

#[derive(Debug, Default)]
pub struct CommandEditorWidget {
    //
}

impl CommandEditorWidget {
    pub fn render(&self, state: &AppState, canvas: &mut Canvas) {
        if state.editing {
            canvas.draw(Token::with_style("-> ", TokenStyle::Bold));
        } else {
            canvas.draw(Token::with_style("   ", TokenStyle::Bold));
        }
        canvas.draw(Token::new("$ "));
        canvas.newline();
    }
}
