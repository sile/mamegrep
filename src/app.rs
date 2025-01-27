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
            widget.render_legend(&self.state, &mut canvas).or_fail()?;
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
                    if let Some(position) = self.state.show_terminal_cursor {
                        self.terminal.show_cursor(position).or_fail()?;
                    } else {
                        self.terminal.hide_cursor().or_fail()?;
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
    hide_legend: bool,
    show_terminal_cursor: Option<TokenPosition>,
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

    fn toggle_all_expansion(&mut self) {
        if self.cursor.line_number.is_some() {
            todo!()
        }

        if self
            .search_result
            .files
            .keys()
            .all(|file| self.collapsed.contains(file))
        {
            self.collapsed.clear();
        } else {
            for file in self.search_result.files.keys() {
                self.collapsed.insert(file.clone());
            }
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
            if !line.matched {
                continue;
            }

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
            self.cursor.line_number = lines.iter().rev().find(|l| l.matched).map(|l| l.number);
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
            if !line.matched {
                continue;
            }

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
        let line_number = self
            .search_result
            .files
            .get(file)
            .expect("infallible")
            .iter()
            .find(|l| l.matched)
            .expect("infallible")
            .number;
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
    fn render_legend(&self, state: &AppState, canvas: &mut Canvas) -> orfail::Result<()>;
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

    fn render_legend(&self, state: &AppState, canvas: &mut Canvas) -> orfail::Result<()> {
        let width = 22;
        if canvas.frame_size().cols < width {
            return Ok(());
        }

        canvas.set_cursor(TokenPosition::row(2));
        if state.hide_legend {
            let col = canvas.frame_size().cols - 11;
            canvas.set_col_offset(col);
            canvas.drawln(Token::new("+- s(h)ow -"));
            return Ok(());
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

        Ok(())
    }

    fn handle_key_event(&mut self, state: &mut AppState, event: KeyEvent) -> orfail::Result<bool> {
        match event.code {
            KeyCode::Char('/') | KeyCode::Char('e') => {
                state.new_widget = Some(Box::new(SearchPatternInputWidget::Pattern));
            }
            KeyCode::Char('a') => {
                state.new_widget = Some(Box::new(SearchPatternInputWidget::AndPattern));
            }
            KeyCode::Char('n') => {
                state.new_widget = Some(Box::new(SearchPatternInputWidget::NotPattern));
            }
            KeyCode::Char('r') => {
                state.new_widget = Some(Box::new(SearchPatternInputWidget::Revision));
            }
            KeyCode::Char('p') => {
                state.new_widget = Some(Box::new(SearchPatternInputWidget::Path));
            }
            KeyCode::Char('h') => {
                state.hide_legend = !state.hide_legend;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('i') => {
                state.grep.ignore_case = !state.grep.ignore_case;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('u') => {
                state.grep.untracked = !state.grep.untracked;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('I') => {
                state.grep.no_index = !state.grep.no_index;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('R') => {
                state.grep.no_recursive = !state.grep.no_recursive;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('w') => {
                state.grep.word_regexp = !state.grep.word_regexp;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('F') if !(state.grep.perl_regexp || state.grep.extended_regexp) => {
                state.grep.fixed_strings = !state.grep.fixed_strings;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('E') if !(state.grep.fixed_strings || state.grep.perl_regexp) => {
                state.grep.extended_regexp = !state.grep.extended_regexp;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('P') if !(state.grep.fixed_strings || state.grep.extended_regexp) => {
                state.grep.perl_regexp = !state.grep.perl_regexp;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('+') => {
                state.grep.context_lines.0 += 1;
                state.regrep().or_fail()?;
            }
            KeyCode::Char('-') if state.grep.context_lines.0 > 0 => {
                state.grep.context_lines.0 -= 1;
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
            KeyCode::Char('T') => {
                state.toggle_all_expansion();
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
        canvas.drawln(Token::with_style(
            format!(
                "SEARCH RESULT ({} hits, {} files)",
                state
                    .search_result
                    .highlight
                    .lines
                    .values()
                    .map(|v| v.len())
                    .sum::<usize>(),
                state.search_result.files.len()
            ),
            TokenStyle::Bold,
        ));

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
            canvas.draw(Token::new(format!(
                " ({hits} hits, {} lines)",
                lines.iter().filter(|l| l.matched).count()
            )));

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
            if !line.matched {
                continue;
            }
            let focused = cursor.is_line_focused(file, line.number);

            if focused {
                canvas.newline();

                // TODO: optimize
                for l in lines {
                    if l.number == line.number {
                        break;
                    }
                    if line.number.get() - l.number.get() <= result.context_lines {
                        canvas.drawln(Token::new(format!(
                            "      {:>width$}|{}",
                            "",
                            l.text,
                            width = result.max_line_width,
                        )));
                    }
                }
            }

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
            canvas.newline();

            if focused {
                // TODO: optimize
                for l in lines {
                    if l.number <= line.number {
                        continue;
                    }
                    if l.number.get() - line.number.get() <= result.context_lines {
                        canvas.drawln(Token::new(format!(
                            "      {:>width$}|{}",
                            "",
                            l.text,
                            width = result.max_line_width,
                        )));
                    } else {
                        break;
                    }
                }
                canvas.newline();
            }
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
        if self.is_line_focused(file, line_number) {
            canvas.draw(Token::new("---> "));
        } else {
            canvas.draw(Token::new("     "));
        }
    }

    pub fn is_line_focused(&self, file: &PathBuf, line_number: NonZeroUsize) -> bool {
        self.file.as_ref() == Some(file) && self.line_number == Some(line_number)
    }
}

#[derive(Debug)]
pub enum SearchPatternInputWidget {
    Pattern,
    AndPattern,
    NotPattern,
    Revision,
    Path,
}

impl Widget for SearchPatternInputWidget {
    fn render(&self, _state: &AppState, _canvas: &mut Canvas) -> orfail::Result<()> {
        Ok(())
    }

    fn render_legend(&self, _state: &AppState, _canvas: &mut Canvas) -> orfail::Result<()> {
        Ok(())
    }

    fn handle_key_event(&mut self, state: &mut AppState, event: KeyEvent) -> orfail::Result<bool> {
        // TODO:
        state.show_terminal_cursor = Some(TokenPosition::row(0));

        match event.code {
            KeyCode::Enter => {
                state.regrep().or_fail()?;
                state.show_terminal_cursor = None;
                return Ok(false);
            }
            KeyCode::Char(c) if !c.is_control() => {
                match self {
                    Self::Pattern => {
                        state.grep.pattern.push(c);
                    }
                    Self::AndPattern => {
                        state.grep.and_pattern.push(c);
                    }
                    Self::NotPattern => {
                        state.grep.not_pattern.push(c);
                    }
                    Self::Revision => {
                        state.grep.revision.push(c);
                    }
                    Self::Path => {
                        state.grep.path.push(c);
                    }
                }
                state.dirty = true;
            }
            KeyCode::Backspace => {
                match self {
                    Self::Pattern => {
                        state.grep.pattern.pop();
                    }
                    Self::AndPattern => {
                        state.grep.and_pattern.pop();
                    }
                    Self::NotPattern => {
                        state.grep.not_pattern.pop();
                    }
                    Self::Revision => {
                        state.grep.revision.pop();
                    }
                    Self::Path => {
                        state.grep.path.pop();
                    }
                }
                state.dirty = true;
            }
            _ => {}
        }
        Ok(true)
    }
}
