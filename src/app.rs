use std::{
    collections::BTreeSet,
    num::NonZeroUsize,
    ops::{RangeFrom, RangeTo},
    path::PathBuf,
};

use crate::{
    canvas::{Canvas, TokenPosition},
    git::{GrepArg, GrepOptions, SearchResult},
    terminal::Terminal,
    widget_command_editor::CommandEditorWidget,
    widget_legend::LegendWidget,
    widget_search_result::{Cursor, SearchResultWidget},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    exit: bool,
    state: AppState,
    legend: LegendWidget,
    command_editor: CommandEditorWidget,
    search_result: SearchResultWidget,
}

impl App {
    pub fn new(initial_options: GrepOptions) -> orfail::Result<Self> {
        let mut this = Self {
            terminal: Terminal::new().or_fail()?,
            exit: false,
            state: AppState::default(),
            legend: LegendWidget::default(),
            command_editor: CommandEditorWidget::default(),
            search_result: SearchResultWidget::default(),
        };

        this.state.grep = initial_options;
        if !this.state.grep.pattern.is_empty() {
            this.state.regrep().or_fail()?;
        } else {
            this.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()))
                .or_fail()?;
        }

        Ok(this)
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.render().or_fail()?;

        while !self.exit {
            let event = self.terminal.next_event().or_fail()?;
            self.handle_event(event).or_fail()?;
        }

        std::mem::drop(self.terminal);

        print!("git");
        for arg in self.state.grep.args(Focus::default()) {
            print!(" {}", arg.quoted_text());
        }
        println!();

        Ok(())
    }

    fn render(&mut self) -> orfail::Result<()> {
        if self.terminal.size().is_empty() {
            return Ok(());
        }

        self.command_editor
            .set_available_cols(self.legend.remaining_cols(self.terminal.size()));

        let mut canvas = Canvas::new(self.terminal.size());
        self.command_editor.render(&self.state, &mut canvas);
        canvas.newline();
        self.search_result.render(&self.state, &mut canvas);
        self.legend.render(&self.state, &mut canvas);

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
        let editing = !matches!(self.state.focus, Focus::SearchResult);
        match event.code {
            KeyCode::Char('q') if !editing => {
                self.exit = true;
            }
            KeyCode::Esc => {
                self.exit = true;
            }
            KeyCode::Char('c') if ctrl => {
                self.exit = true;
            }
            KeyCode::Char('H') if !editing => {
                self.legend.hide = !self.legend.hide;
                self.state.dirty = true;
            }
            _ => {
                let old_focus = self.state.focus;
                if editing {
                    self.command_editor
                        .handle_key_event(&mut self.state, event)
                        .or_fail()?;
                } else {
                    self.search_result
                        .handle_key_event(&mut self.state, event)
                        .or_fail()?;
                }

                if old_focus != self.state.focus {
                    self.command_editor.handle_focus_change(&mut self.state);
                }
            }
        }

        if self.state.dirty {
            self.render().or_fail()?;
            self.command_editor.update_cursor_position(&mut self.state);

            if let Some(position) = self.state.show_terminal_cursor {
                self.terminal.show_cursor(position).or_fail()?;
            } else {
                self.terminal.hide_cursor().or_fail()?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    #[default]
    SearchResult,
    Pattern,
    AndPattern,
    NotPattern,
    Revision,
    Path,
}

impl Focus {
    pub fn is_editing(self) -> bool {
        !matches!(self, Self::SearchResult)
    }
}

#[derive(Debug, Default)]
pub struct AppState {
    pub grep: GrepOptions,
    pub dirty: bool,
    pub search_result: SearchResult,
    pub cursor: Cursor,
    pub collapsed: BTreeSet<PathBuf>,
    pub show_terminal_cursor: Option<TokenPosition>,
    pub focus: Focus,
}

impl AppState {
    pub fn can_cursor_up(&self) -> bool {
        if self.cursor.is_file_level() {
            self.peek_cursor_up_file().is_some()
        } else if self.cursor.is_line_level() {
            self.peek_cursor_up_line().is_some()
        } else {
            false
        }
    }

    pub fn can_cursor_down(&self) -> bool {
        if self.cursor.is_file_level() {
            self.peek_cursor_down_file().is_some()
        } else if self.cursor.is_line_level() {
            self.peek_cursor_down_line().is_some()
        } else {
            false
        }
    }

    pub fn focused_arg_mut(&mut self) -> Option<&mut GrepArg> {
        match self.focus {
            Focus::SearchResult => None,
            Focus::Pattern => Some(&mut self.grep.pattern),
            Focus::AndPattern => Some(&mut self.grep.and_pattern),
            Focus::NotPattern => Some(&mut self.grep.not_pattern),
            Focus::Revision => Some(&mut self.grep.revision),
            Focus::Path => Some(&mut self.grep.path),
        }
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
        self.dirty = true;
    }

    pub fn flip_grep_flag<F>(&mut self, f: F) -> orfail::Result<()>
    where
        F: FnOnce(&mut GrepOptions) -> &mut bool,
    {
        let flag = f(&mut self.grep);
        *flag = !*flag;
        self.regrep().or_fail()
    }

    pub fn regrep(&mut self) -> orfail::Result<()> {
        let result = self.grep.call().or_fail();
        match result {
            Ok(result) => {
                self.search_result = result;
            }
            Err(e) => {
                if let Some(result) = self.grep.get_error_result() {
                    self.search_result = result;
                } else {
                    return Err(e);
                }
            }
        }
        self.dirty = true;
        self.reset_cursor();
        Ok(())
    }

    pub fn toggle_expansion(&mut self) {
        if self.cursor.is_line_level() {
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

    pub fn toggle_all_expansion(&mut self) {
        fn can_collapse(cursor: &Cursor, file: &PathBuf) -> bool {
            cursor.is_file_level() || cursor.file.as_ref() != Some(file)
        }

        let target_files = self
            .search_result
            .files
            .keys()
            .filter(|file| can_collapse(&self.cursor, file));
        if target_files
            .clone()
            .all(|file| self.collapsed.contains(file))
        {
            self.collapsed.clear();
        } else {
            self.collapsed.extend(target_files.cloned());
        }

        self.dirty = true;
    }

    pub fn cursor_up(&mut self) {
        if self.cursor.is_file_level() {
            self.cursor_up_file();
        } else if self.cursor.is_line_level() {
            self.cursor_up_line();
        }
    }

    fn peek_cursor_up_file(&self) -> Option<&PathBuf> {
        let file = self.cursor.file.as_ref().expect("infallible");
        self.search_result
            .files
            .range::<PathBuf, RangeTo<_>>(..file)
            .next_back()
            .map(|(k, _)| k)
    }

    fn cursor_up_file(&mut self) {
        if let Some(new) = self.peek_cursor_up_file().cloned() {
            self.cursor.file = Some(new);
            self.dirty = true;
        }
    }

    fn peek_cursor_up_line(&self) -> Option<(&PathBuf, NonZeroUsize)> {
        let file = self.cursor.file.as_ref()?;
        let line_number = self.cursor.line_number?;
        let lines = self.search_result.files.get(file).expect("infallible");

        if let Some(new_line) = lines
            .iter()
            .rfind(|line| line.hit && line.number < line_number)
        {
            Some((file, new_line.number))
        } else if let Some(new_file) = self.peek_cursor_up_file() {
            let lines = self.search_result.files.get(new_file).expect("infallible");
            let new_line = lines.iter().rfind(|line| line.hit).expect("infallible");
            Some((new_file, new_line.number))
        } else {
            None
        }
    }

    fn cursor_up_line(&mut self) {
        if let Some((file, line_number)) = self.peek_cursor_up_line() {
            let file = file.clone();
            self.collapsed.remove(&file);
            self.cursor.file = Some(file);
            self.cursor.line_number = Some(line_number);
            self.dirty = true;
        }
    }

    pub fn cursor_down(&mut self) {
        if self.cursor.is_file_level() {
            self.cursor_down_file();
        } else if self.cursor.is_line_level() {
            self.cursor_down_line();
        }
    }

    fn peek_cursor_down_line(&self) -> Option<(&PathBuf, NonZeroUsize)> {
        let file = self.cursor.file.as_ref()?;
        let line_number = self.cursor.line_number?;
        let lines = self.search_result.files.get(file).expect("infallible");

        if let Some(new_line) = lines
            .iter()
            .find(|line| line.hit && line.number > line_number)
        {
            Some((file, new_line.number))
        } else if let Some(new_file) = self.peek_cursor_down_file() {
            let lines = self.search_result.files.get(new_file).expect("infallible");
            let new_line = lines.iter().find(|line| line.hit).expect("infallible");
            Some((new_file, new_line.number))
        } else {
            None
        }
    }

    fn cursor_down_line(&mut self) {
        if let Some((file, line_number)) = self.peek_cursor_down_line() {
            let file = file.clone();
            self.collapsed.remove(&file);
            self.cursor.file = Some(file);
            self.cursor.line_number = Some(line_number);
            self.dirty = true;
        }
    }

    fn peek_cursor_down_file(&self) -> Option<&PathBuf> {
        let file = self.cursor.file.as_ref().expect("infallible");
        self.search_result
            .files
            .range::<PathBuf, RangeFrom<_>>(file..)
            .nth(1)
            .map(|(k, _)| k)
    }

    fn cursor_down_file(&mut self) {
        if let Some(new) = self.peek_cursor_down_file().cloned() {
            self.cursor.file = Some(new);
            self.dirty = true;
        }
    }

    pub fn cursor_right(&mut self) {
        if self.search_result.is_empty() | self.cursor.is_line_level() {
            return;
        }

        let file = self.cursor.file.as_ref().expect("infallible");
        let line_number = self
            .search_result
            .files
            .get(file)
            .expect("infallible")
            .iter()
            .find(|l| l.hit)
            .expect("infallible")
            .number;
        self.cursor.line_number = Some(line_number);
        self.collapsed.remove(file);
        self.dirty = true;
    }

    pub fn cursor_left(&mut self) {
        if self.cursor.is_line_level() {
            self.cursor.line_number = None;
            self.dirty = true;
        }
    }

    fn reset_cursor(&mut self) {
        if self.search_result.is_empty() {
            self.cursor = Cursor::default();
            return;
        }

        let Some(old_file) = &self.cursor.file else {
            let new_file = self.search_result.files.keys().next().cloned();
            self.cursor.file = new_file;
            return;
        };

        if !self.search_result.files.contains_key(old_file) {
            let new_file = self
                .search_result
                .files
                .range::<PathBuf, RangeTo<_>>(..old_file)
                .rev()
                .chain(
                    self.search_result
                        .files
                        .range::<PathBuf, RangeFrom<_>>(old_file..),
                )
                .next()
                .map(|(k, _)| k.clone());
            self.cursor.file = new_file;
            self.cursor.line_number = None;
            return;
        }

        let Some(old_line_number) = self.cursor.line_number else {
            return;
        };
        let file = self.cursor.file.as_ref().expect("infallible");
        let lines = self.search_result.files.get(file).expect("infallible");
        self.cursor.line_number = lines
            .iter()
            .rfind(|line| line.hit && old_line_number >= line.number)
            .or_else(|| lines.iter().find(|line| line.hit))
            .map(|line| line.number);
    }
}
