use std::{
    collections::BTreeSet,
    num::NonZeroUsize,
    ops::{RangeFrom, RangeTo},
    path::PathBuf,
};

use crate::{
    canvas::{Canvas, Token, TokenPosition, TokenStyle},
    git::{GrepOptions, MatchLine, SearchResult},
    terminal::Terminal,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    exit: bool,
    frame_row_start: usize,
    state: AppState,
    // TODO: Add DetailedWidget (or MatchContextWidget)
    widgets: Vec<Box<dyn 'static + Widget>>,
}

impl App {
    pub fn new() -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            exit: false,
            frame_row_start: 0,
            state: AppState::default(),
            widgets: vec![Box::new(MainWidget {
                tree: Tree::default(),
            })],
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.render().or_fail()?;

        while !self.exit {
            let event = self.terminal.next_event().or_fail()?;
            self.handle_event(event).or_fail()?;
        }

        std::mem::drop(self.terminal);
        println!("{}", self.state.grep.command_string());

        Ok(())
    }

    fn render(&mut self) -> orfail::Result<()> {
        if self.terminal.size().is_empty() {
            return Ok(());
        }

        let mut canvas = Canvas::new(self.frame_row_start, self.terminal.size());
        for widget in &self.widgets {
            widget.render(&self.state, &mut canvas).or_fail()?;
        }
        if let Some(widget) = self.widgets.last() {
            widget.render_legend(&mut canvas).or_fail()?;
        }
        self.terminal.draw_frame(canvas.into_frame()).or_fail()?;

        self.state.dirty = false;
        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> orfail::Result<()> {
        match event {
            Event::FocusGained => Ok(()),
            Event::FocusLost => Ok(()),
            Event::Key(event) => self.handle_key_event(event).or_fail(),
            Event::Mouse(_) => Ok(()),
            Event::Paste(_) => Ok(()),
            Event::Resize(_, _) => self.render().or_fail(),
        }
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> orfail::Result<()> {
        if event.kind != KeyEventKind::Press {
            return Ok(());
        }

        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.exit = true;
            }
            KeyCode::Char('c') if ctrl => {
                self.exit = true;
            }
            _ => {
                if let Some(widget) = self.widgets.last_mut() {
                    if !widget.handle_key_event(&mut self.state, event).or_fail()? {
                        self.widgets.pop();
                        self.state.dirty = true;
                    }
                    if let Some(widget) = self.state.new_widget.take() {
                        self.widgets.push(widget);
                        self.state.dirty = true;
                    }
                }
            }
        }

        if self.state.dirty {
            self.render().or_fail()?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct AppState {
    grep: GrepOptions,
    new_widget: Option<Box<dyn 'static + Widget>>,
    dirty: bool,
    search_result: SearchResult,
    cursor: Cursor,
    collapsed: BTreeSet<PathBuf>,
}

impl AppState {
    pub fn regrep(&mut self) -> orfail::Result<()> {
        self.search_result = self.grep.call().or_fail()?;
        self.dirty = true;
        self.reset_cursor();
        Ok(())
    }

    fn toggle_expansion(&mut self) {
        if self.cursor.line_number.is_some() {
            return;
        }

        let Some(file) = &self.cursor.file else {
            return;
        };
        if !self.collapsed.remove(file) {
            self.collapsed.insert(file.clone());
        }
        self.dirty = true;
    }

    fn cursor_up(&mut self) {
        if self.search_result.files.is_empty() {
            return;
        }

        if self.cursor.line_number.is_some() {
            self.cursor_up_line();
        } else {
            self.cursor_up_file();
        }
    }

    fn cursor_up_file(&mut self) {
        let file = self.cursor.file.as_ref().expect("infallible");
        let new = self
            .search_result
            .files
            .range::<PathBuf, RangeTo<_>>(..file)
            .rev()
            .next()
            .map(|(k, _)| k.clone());
        if new.is_some() {
            self.cursor.file = new;
            self.dirty = true;
        }
    }

    fn cursor_up_line(&mut self) {
        let file = self.cursor.file.as_ref().expect("infallible");
        let line_number = self.cursor.line_number.expect("infallible");

        let lines = self.search_result.files.get(file).expect("infallible");

        // TODO: optimize
        for line in lines.iter().rev() {
            if line_number > line.number {
                self.cursor.line_number = Some(line.number);
                self.dirty = true;
                return;
            }
        }

        let current = self.cursor.clone();
        self.cursor_left();
        self.cursor_up();
        self.cursor_right();
        if current.file == self.cursor.file {
            self.cursor = current;
        } else {
            let file = self.cursor.file.as_ref().expect("infallible");
            let lines = self.search_result.files.get(file).expect("infallible");
            self.cursor.line_number = lines.last().map(|l| l.number);
        }
    }

    fn cursor_down(&mut self) {
        if self.search_result.files.is_empty() {
            return;
        }

        if self.cursor.line_number.is_some() {
            self.cursor_down_line();
        } else {
            self.cursor_down_file();
        }
    }

    fn cursor_down_line(&mut self) {
        let file = self.cursor.file.as_ref().expect("infallible");
        let line_number = self.cursor.line_number.expect("infallible");

        let lines = self.search_result.files.get(file).expect("infallible");

        // TODO: optimize
        for line in lines {
            if line_number < line.number {
                self.cursor.line_number = Some(line.number);
                self.dirty = true;
                return;
            }
        }

        let current = self.cursor.clone();
        self.cursor_left();
        self.cursor_down();
        self.cursor_right();
        if current.file == self.cursor.file {
            self.cursor = current;
        }
    }

    fn cursor_down_file(&mut self) {
        let file = self.cursor.file.as_ref().expect("infallible");
        let new = self
            .search_result
            .files
            .range::<PathBuf, RangeFrom<_>>(file..)
            .nth(1)
            .map(|(k, _)| k.clone());
        if new.is_some() {
            self.cursor.file = new;
            self.dirty = true;
        }
    }

    fn cursor_right(&mut self) {
        if self.search_result.files.is_empty() | self.cursor.line_number.is_some() {
            return;
        }

        let file = self.cursor.file.as_ref().expect("infallible");
        let line_number = self.search_result.files.get(file).expect("infallible")[0].number;
        self.cursor.line_number = Some(line_number);
        self.dirty = true;
    }

    fn cursor_left(&mut self) {
        self.cursor.line_number = None;
        self.dirty = true;
    }

    fn reset_cursor(&mut self) {
        if self.search_result.files.is_empty() {
            self.cursor = Cursor::default();
            return;
        }

        if self.cursor.line_number.is_some() {
            todo!();
        }

        if let Some(f) = &self.cursor.file {
            if !self.search_result.files.contains_key(f) {
                self.cursor.line_number = None;
                let new = self
                    .search_result
                    .files
                    .range::<PathBuf, RangeTo<_>>(..f)
                    .rev()
                    .chain(self.search_result.files.range::<PathBuf, RangeFrom<_>>(f..))
                    .next()
                    .map(|(k, _)| k.clone());
                self.cursor.file = new;
            }
        } else {
            let new = self.search_result.files.keys().next().cloned();
            self.cursor.file = new;
        }

        let file = self.cursor.file.as_ref().expect("infallible");
        let Some(line_number) = self.cursor.line_number else {
            return;
        };
        let lines = self.search_result.files.get(file).expect("infallible");
        if let Err(i) = lines.binary_search_by_key(&line_number, |x| x.number) {
            let line = lines.get(i).unwrap_or(lines.last().expect("infallible"));
            self.cursor.line_number = Some(line.number);
        }
    }
}

pub trait Widget: std::fmt::Debug {
    fn render(&self, state: &AppState, canvas: &mut Canvas) -> orfail::Result<()>;
    fn render_legend(&self, canvas: &mut Canvas) -> orfail::Result<()>;
    fn handle_key_event(&mut self, state: &mut AppState, event: KeyEvent) -> orfail::Result<bool>;
}

#[derive(Debug)]
pub struct MainWidget {
    pub tree: Tree,
}

impl Widget for MainWidget {
    fn render(&self, state: &AppState, canvas: &mut Canvas) -> orfail::Result<()> {
        canvas.drawln(Token::new(state.grep.command_string()));
        canvas.drawln(Token::new(
            std::iter::repeat_n('-', canvas.frame_size().cols).collect::<String>(),
        ));

        self.tree.render(canvas, state);

        Ok(())
    }

    fn render_legend(&self, _canvas: &mut Canvas) -> orfail::Result<()> {
        Ok(())
    }

    fn handle_key_event(&mut self, state: &mut AppState, event: KeyEvent) -> orfail::Result<bool> {
        match event.code {
            KeyCode::Char('/') => {
                state.new_widget = Some(Box::new(SearchPatternInputWidget {}));
            }
            KeyCode::Char('i') => {
                state.grep.ignore_case = !state.grep.ignore_case;
                state.regrep().or_fail()?;
            }
            KeyCode::Up => {
                state.cursor_up();
            }
            KeyCode::Down => {
                state.cursor_down();
            }
            KeyCode::Right => {
                state.cursor_right();
            }
            KeyCode::Left => {
                state.cursor_left();
            }
            KeyCode::Char('t') => {
                state.toggle_expansion();
            }
            _ => {}
        }
        Ok(true)
    }
}

#[derive(Debug, Default)]
pub struct Tree {}

impl Tree {
    fn render(&self, canvas: &mut Canvas, state: &AppState) {
        for (file, lines) in &state.search_result.files {
            state.cursor.render_for_file(canvas, file);

            let hits = state
                .search_result
                .highlight
                .lines
                .get(file)
                .map(|v| v.values().map(|v| v.len()).sum::<usize>())
                .unwrap_or_default();
            canvas.draw(Token::with_style(
                format!("{}", file.display()),
                TokenStyle::Underlined,
            ));
            canvas.draw(Token::new(format!(" ({} lines, {hits} hits)", lines.len())));

            if state.collapsed.contains(file) {
                canvas.drawln(Token::new("…"));
            } else {
                canvas.newline();
                self.render_lines(canvas, &state.cursor, &state.search_result, file, lines);
            }
        }
    }

    fn render_lines(
        &self,
        canvas: &mut Canvas,
        cursor: &Cursor,
        result: &SearchResult,
        file: &PathBuf,
        lines: &[MatchLine],
    ) {
        for line in lines {
            cursor.render_for_line(canvas, file, line.number);

            // TODO: rename var
            let matched_columns = result
                .highlight
                .lines
                .get(file)
                .and_then(|v| v.get(&line.number))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            canvas.draw(Token::new(format!(
                "[{:>width$}]",
                line.number,
                width = result.max_line_width
            )));

            let base = canvas.cursor();
            canvas.draw(Token::new(format!("{}", line.text)));

            let mut offset = 0;
            for matched_text in matched_columns {
                // TODO: Consider multi byte char
                let i = offset + line.text[offset..].find(matched_text).expect("TODO");
                let s = matched_text;
                offset = i + matched_text.len();
                canvas.draw_at(
                    TokenPosition {
                        row: base.row,

                        col: base.col + i,
                    },
                    Token::with_style(s, TokenStyle::Reverse),
                );
            }
            // TODO: optimize

            canvas.newline();
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Cursor {
    pub file: Option<PathBuf>,
    pub line_number: Option<NonZeroUsize>,
}

impl Cursor {
    pub fn render_for_file(&self, canvas: &mut Canvas, file: &PathBuf) {
        if self.line_number.is_some() {
            canvas.draw(Token::new("   "));
        } else if self.file.as_ref() == Some(file) {
            canvas.draw(Token::new("-> "));
        } else {
            canvas.draw(Token::new("   "));
        }
    }

    pub fn render_for_line(&self, canvas: &mut Canvas, file: &PathBuf, line_number: NonZeroUsize) {
        if self.file.as_ref() == Some(file) && self.line_number == Some(line_number) {
            canvas.draw(Token::new("---> "));
        } else {
            canvas.draw(Token::new("     "));
        }
    }
}

#[derive(Debug)]
pub struct SearchPatternInputWidget {}

impl Widget for SearchPatternInputWidget {
    fn render(&self, _state: &AppState, canvas: &mut Canvas) -> orfail::Result<()> {
        canvas.drawln(Token::new("Grep: "));
        Ok(())
    }

    fn render_legend(&self, _canvas: &mut Canvas) -> orfail::Result<()> {
        Ok(())
    }

    fn handle_key_event(&mut self, state: &mut AppState, event: KeyEvent) -> orfail::Result<bool> {
        match event.code {
            KeyCode::Enter => {
                state.regrep().or_fail()?;
                return Ok(false);
            }
            KeyCode::Char(c) if !c.is_control() => {
                state.grep.pattern.push(c);
                state.dirty = true;
            }
            KeyCode::Backspace => {
                state.grep.pattern.pop();
                state.dirty = true;
            }
            _ => {}
        }
        Ok(true)
    }
}
