use std::{
    collections::BTreeSet,
    num::NonZeroUsize,
    ops::{RangeFrom, RangeTo},
    path::PathBuf,
};

use mame::action::{Binding, BindingConfig, BindingContextName};
use orfail::OrFail;
use tuinix::{Terminal, TerminalEvent, TerminalPosition};

use crate::{
    action::Action,
    canvas::Canvas,
    git::{GrepArg, GrepOptions, SearchResult},
    widget_command_editor::CommandEditorWidget,
    widget_legend::LegendWidget,
    widget_search_result::{Cursor, SearchResultWidget},
};

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    config: BindingConfig<Action>,
    context: BindingContextName,
    exit: bool,
    state: AppState,
    legend: LegendWidget,
    command_editor: CommandEditorWidget,
    search_result: SearchResultWidget,
    preview: Option<mame::preview::TextPreview>,
}

impl App {
    pub fn new(
        initial_options: GrepOptions,
        config: BindingConfig<Action>,
    ) -> orfail::Result<Self> {
        let binding_for_editing = config
            .all_bindings()
            .flat_map(|(_, bindings)| bindings.iter())
            .find(|b| matches!(b.action, Some(Action::SetFocus(Focus::Pattern))))
            .cloned();

        let mut this = Self {
            terminal: Terminal::new().or_fail()?,
            context: config.initial_context().clone(),
            config,
            exit: false,
            state: AppState::default(),
            legend: LegendWidget::default(),
            command_editor: CommandEditorWidget::default(),
            search_result: SearchResultWidget::default(),
            preview: None,
        };

        this.state.grep = initial_options;
        if !this.state.grep.pattern.is_empty() {
            this.state.regrep().or_fail()?;
        } else if let Some(b) = binding_for_editing {
            this.handle_binding(b).or_fail()?;
        }

        Ok(this)
    }

    pub fn run(mut self) -> orfail::Result<()> {
        if let Some(action) = self.config.setup_action().cloned() {
            self.handle_action(action).or_fail()?;
        }
        self.render().or_fail()?;

        while !self.exit {
            let Some(event) = self.terminal.poll_event(&[], &[], None).or_fail()? else {
                continue;
            };
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
            .set_available_cols(self.legend.remaining_cols(
                self.terminal.size(),
                self.config.get_bindings(&self.context).or_fail()?,
                &self.state,
            ));

        let mut canvas = Canvas::new(self.terminal.size());
        self.command_editor.render(&self.state, &mut canvas);
        canvas.newline();
        self.search_result.render(&self.state, &mut canvas);

        self.command_editor.update_cursor_position(&mut self.state);
        self.terminal.set_cursor(self.state.show_terminal_cursor);

        let mut frame = canvas.into_frame().into_terminal_frame();
        if let Some(preview) = &mut self.preview {
            preview.render(&mut frame).or_fail()?;
        }
        self.legend
            .render(
                &mut frame,
                self.config.get_bindings(&self.context).or_fail()?,
                &self.state,
            )
            .or_fail()?;
        self.terminal.draw(frame).or_fail()?;

        Ok(())
    }

    fn handle_action(&mut self, action: Action) -> orfail::Result<()> {
        match action {
            Action::Quit => {
                self.exit = true;
            }
            Action::ToggleLegend => {
                self.legend.hide = !self.legend.hide;
            }
            Action::InitLegend {
                label_show,
                label_hide,
                hide,
            } => {
                self.legend.label_show = label_show;
                self.legend.label_hide = label_hide;
                self.legend.hide = hide;
            }
            Action::ExecuteCommand(command) => {
                self.execute_command(&command).or_fail()?;
            }
            _ => {
                let old_focus = self.state.focus;
                if self.state.focus.is_editing() {
                    self.command_editor
                        .handle_action(&mut self.state, action)
                        .or_fail()?;
                } else {
                    self.search_result
                        .handle_action(&mut self.state, action)
                        .or_fail()?;
                }

                if old_focus != self.state.focus {
                    self.command_editor.handle_focus_change(&mut self.state);
                }
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, event: TerminalEvent) -> orfail::Result<()> {
        match event {
            TerminalEvent::Resize(_) => self.render().or_fail(),
            TerminalEvent::Input(input) => {
                if let tuinix::TerminalInput::Key(tuinix::KeyInput {
                    code: tuinix::KeyCode::Char(c),
                    ..
                }) = input
                {
                    self.state.last_input_char = c;
                }
                let bindings = self.config.get_bindings(&self.context).or_fail()?;
                if let Some(binding) = bindings.iter().find(|b| b.matches(input)).cloned() {
                    self.handle_binding(binding).or_fail()?;
                    self.render().or_fail()?;
                }
                Ok(())
            }
            TerminalEvent::FdReady { .. } => Err(orfail::Failure::new("bug")),
        }
    }

    fn handle_binding(&mut self, binding: Binding<Action>) -> orfail::Result<()> {
        if let Some(action) = binding.action {
            self.handle_action(action).or_fail()?;
        }
        if let Some(context) = binding.context {
            self.context = context;
        }
        Ok(())
    }

    fn execute_command(&mut self, command: &mame::command::ExternalCommand) -> orfail::Result<()> {
        let executing_pane = mame::preview::TextPreviewPane::new(
            "executing",
            &format!("$ {}", command.command_line()),
        );
        self.preview = Some(mame::preview::TextPreview::new(Some(executing_pane), None));
        self.render().or_fail()?;

        let mut command = command.clone();

        let mut grep_command = "git".to_owned();
        for arg in self.state.grep.args(Focus::default()) {
            grep_command.push(' ');
            grep_command.push_str(&arg.quoted_text());
        }
        command
            .envs
            .insert("MAMEGREP_GREP_COMMAND".to_owned(), grep_command);

        if let Some(file) = &self.state.cursor.file {
            command
                .envs
                .insert("MAMEGREP_FILE".to_owned(), file.display().to_string());
        }
        if let Some(line_number) = self.state.cursor.line_number {
            command
                .envs
                .insert("MAMEGREP_LINE".to_owned(), line_number.to_string());
        }
        let output = command.execute().or_fail()?;

        // If the command was successful, re-run the grep to refresh results
        if output.status.success() {
            self.state.regrep().or_fail()?;
        }

        let stdout_pane =
            mame::preview::TextPreviewPane::new("stdout", &String::from_utf8_lossy(&output.stdout));
        let stderr_pane =
            mame::preview::TextPreviewPane::new("stderr", &String::from_utf8_lossy(&output.stderr));
        self.preview = Some(mame::preview::TextPreview::new(
            Some(stdout_pane),
            Some(stderr_pane),
        ));
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
    pub search_result: SearchResult,
    pub cursor: Cursor,
    pub collapsed: BTreeSet<PathBuf>,
    pub show_terminal_cursor: Option<TerminalPosition>,
    pub focus: Focus,
    pub last_input_char: char,
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
    }

    pub fn cursor_left(&mut self) {
        if self.cursor.is_line_level() {
            self.cursor.line_number = None;
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
