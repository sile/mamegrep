use std::{borrow::Cow, collections::BTreeMap, num::NonZeroUsize, path::PathBuf, process::Command};

use orfail::OrFail;

use crate::app::Focus;

#[derive(Debug)]
enum Mode {
    External,
    Parsing,
    Highlight,
}

#[derive(Debug, Default, Clone)]
pub struct Highlight {
    pub lines: BTreeMap<PathBuf, BTreeMap<NonZeroUsize, Vec<String>>>,
}

impl Highlight {
    fn parse(s: &str) -> orfail::Result<Self> {
        let mut lines = BTreeMap::<_, BTreeMap<_, Vec<_>>>::new();
        let mut current = PathBuf::new();
        for line in s.lines() {
            if let Some(m) = Line::parse(line) {
                lines
                    .get_mut(&current)
                    .or_fail()?
                    .entry(m.number)
                    .or_default()
                    .push(m.text);
            } else {
                current = PathBuf::from(line);
                lines.insert(current.clone(), BTreeMap::new());
            }
        }
        Ok(Self { lines })
    }
}

#[derive(Debug, Default, Clone)]
pub struct SearchResult {
    pub files: BTreeMap<PathBuf, Vec<Line>>,
    pub max_line_width: usize,
    pub highlight: Highlight,
    pub error: Option<String>,
}

impl SearchResult {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn hit_files(&self) -> usize {
        self.files.len()
    }

    pub fn hit_lines(&self) -> usize {
        self.highlight
            .lines
            .values()
            .map(|lines| lines.len())
            .sum::<usize>()
    }

    pub fn hit_texts(&self) -> usize {
        self.highlight
            .lines
            .values()
            .flat_map(|lines| lines.values().map(|texts| texts.len()))
            .sum::<usize>()
    }

    pub fn hit_texts_in_file(&self, file: &PathBuf) -> usize {
        self.highlight
            .lines
            .get(file)
            .map(|v| v.values().map(|v| v.len()).sum::<usize>())
            .unwrap_or_default()
    }

    pub fn hit_lines_in_file(&self, file: &PathBuf) -> usize {
        self.files
            .get(file)
            .map(|lines| lines.iter().filter(|l| l.hit).count())
            .unwrap_or_default()
    }

    pub fn hit_texts_in_line(&self, file: &PathBuf, line_number: NonZeroUsize) -> &[String] {
        self.highlight
            .lines
            .get(file)
            .and_then(|v| v.get(&line_number))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    fn parse(s: &str, highlight: Highlight) -> orfail::Result<Self> {
        let mut files = BTreeMap::<_, Vec<_>>::new();
        let mut current = PathBuf::new();
        let mut max_line_width = 1;
        for line in s.lines() {
            if line == "--" {
                continue;
            }

            if let Some(line) = Line::parse(line) {
                max_line_width = max_line_width.max(line.number.to_string().len());
                files.get_mut(&current).or_fail()?.push(line);
            } else {
                current = PathBuf::from(line);
                files.insert(current.clone(), Vec::new());
            }
        }
        Ok(Self {
            files,
            max_line_width,
            highlight,
            error: None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub number: NonZeroUsize,
    pub text: String,
    pub hit: bool,
}

impl Line {
    fn parse(line: &str) -> Option<Self> {
        for (i, c) in line.char_indices() {
            match c {
                ':' => {
                    let number = line[..i].parse().ok()?;
                    return Some(Self {
                        number,
                        text: line[i + 1..].to_owned(),
                        hit: true,
                    });
                }
                '-' => {
                    let number = line[..i].parse().ok()?;
                    return Some(Self {
                        number,
                        text: line[i + 1..].to_owned(),
                        hit: false,
                    });
                }
                '0'..='9' => {}
                _ => return None,
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContextLines(pub usize);

impl ContextLines {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(9);
}

impl Default for ContextLines {
    fn default() -> Self {
        Self(4)
    }
}

#[derive(Debug, Clone)]
pub struct GrepArg {
    pub kind: GrepArgKind,
    pub text: String,
    pub multiline_head: bool,
}

impl GrepArg {
    fn new(kind: GrepArgKind) -> Self {
        Self {
            kind,
            text: String::new(),
            multiline_head: false,
        }
    }

    fn line_breakable(mut self) -> Self {
        self.multiline_head = true;
        self
    }

    pub fn insert(&mut self, i: usize, c: char) {
        self.text.insert(i, c);
    }

    pub fn remove(&mut self, i: usize) -> Option<char> {
        (i < self.text.len()).then(|| self.text.remove(i))
    }

    pub fn next_char(&self, i: usize) -> Option<char> {
        self.text[i..].chars().next()
    }

    pub fn prev_char(&self, i: usize) -> Option<char> {
        self.text[..i].chars().next_back()
    }

    pub fn is_enabled(&self, focus: Focus) -> bool {
        !self.is_empty() || self.kind.is_focused(focus)
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn maybe_quoted_text(&self, focus: Focus) -> Cow<'_, str> {
        if self.kind.is_focused(focus) || self.kind == GrepArgKind::Other {
            Cow::Borrowed(&self.text)
        } else {
            self.quoted_text()
        }
    }

    pub fn width(&self, focus: Focus) -> usize {
        mame::terminal::str_cols(&self.maybe_quoted_text(focus))
    }

    pub fn quoted_text(&self) -> Cow<'_, str> {
        if self.text.is_empty() {
            return Cow::Borrowed("''");
        } else if !self.text.contains([
            ' ', '\'', '$', '|', '&', '(', ')', '>', '<', '*', '?', '!', ';', '\\', '"',
        ]) {
            return Cow::Borrowed(&self.text);
        }

        let mut quoted = String::new();
        quoted.push('\'');
        for c in self.text.chars() {
            if c == '\'' {
                quoted.push_str(r#"'\'"#);
            } else {
                quoted.push(c);
            }
        }
        quoted.push('\'');
        Cow::Owned(quoted)
    }

    fn other(s: &str) -> Self {
        Self {
            kind: GrepArgKind::Other,
            text: s.to_string(),
            multiline_head: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrepArgKind {
    Pattern,
    AndPattern,
    NotPattern,
    Revision,
    Path,
    Other,
}

impl GrepArgKind {
    pub fn is_focused(self, focus: Focus) -> bool {
        matches!(
            (self, focus),
            (Self::Pattern, Focus::Pattern)
                | (Self::AndPattern, Focus::AndPattern)
                | (Self::NotPattern, Focus::NotPattern)
                | (Self::Revision, Focus::Revision)
                | (Self::Path, Focus::Path)
        )
    }
}

#[derive(Debug, Clone)]
pub struct GrepOptions {
    pub pattern: GrepArg,
    pub and_pattern: GrepArg,
    pub not_pattern: GrepArg,
    pub revision: GrepArg,
    pub path: GrepArg,
    pub ignore_case: bool,
    pub untracked: bool,
    pub no_index: bool,
    pub no_recursive: bool,
    pub word_regexp: bool,
    pub extended_regexp: bool,
    pub fixed_strings: bool,
    pub perl_regexp: bool,
    pub context_lines: ContextLines,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            pattern: GrepArg::new(GrepArgKind::Pattern),
            and_pattern: GrepArg::new(GrepArgKind::AndPattern),
            not_pattern: GrepArg::new(GrepArgKind::NotPattern),
            revision: GrepArg::new(GrepArgKind::Revision),
            path: GrepArg::new(GrepArgKind::Path),
            ignore_case: false,
            untracked: false,
            no_index: false,
            no_recursive: false,
            word_regexp: false,
            extended_regexp: false,
            fixed_strings: false,
            perl_regexp: false,
            context_lines: ContextLines::default(),
        }
    }
}

impl GrepOptions {
    pub fn args(&self, focus: Focus) -> Vec<GrepArg> {
        self.build_grep_args(Mode::External, focus)
    }

    pub fn get_error_result(&self) -> Option<SearchResult> {
        let args = self.build_grep_args(Mode::External, Focus::SearchResult);
        let args = args.iter().map(|s| s.text.as_str()).collect::<Vec<_>>();
        let output = Command::new("git").args(args).output().ok()?;
        if output.stderr.is_empty() {
            return None;
        }
        String::from_utf8(output.stderr)
            .ok()
            .map(|error| SearchResult {
                error: Some(error.trim().to_owned()),
                ..Default::default()
            })
    }

    pub fn call(&self) -> orfail::Result<SearchResult> {
        if self.pattern.is_empty() {
            return Ok(SearchResult::default());
        }

        std::thread::scope(|s| {
            let handle0 = s.spawn(|| {
                let args = self.build_grep_args(Mode::Highlight, Focus::SearchResult);
                let args = args.iter().map(|s| s.text.as_str()).collect::<Vec<_>>();
                let output = call(&args, false).or_fail()?;
                Highlight::parse(&output).or_fail()
            });
            let handle1 = s.spawn(|| {
                let args = self.build_grep_args(Mode::Parsing, Focus::SearchResult);
                let args = args.iter().map(|s| s.text.as_str()).collect::<Vec<_>>();
                let output = call(&args, false).or_fail()?;
                SearchResult::parse(&output, Highlight::default()).or_fail()
            });

            let highlight = handle0
                .join()
                .unwrap_or_else(|e| std::panic::resume_unwind(e))
                .or_fail()?;
            let mut search_result = handle1
                .join()
                .unwrap_or_else(|e| std::panic::resume_unwind(e))
                .or_fail()?;
            search_result.highlight = highlight;
            Ok(search_result)
        })
    }

    fn build_grep_args(&self, mode: Mode, focus: Focus) -> Vec<GrepArg> {
        let mut args = vec![GrepArg::other("grep")];

        let mut flags = "-nI".to_string();
        if self.ignore_case {
            flags.push('i');
        }
        if self.word_regexp {
            flags.push('w');
        }
        if self.extended_regexp {
            flags.push('E');
        }
        if self.fixed_strings {
            flags.push('F');
        }
        if self.perl_regexp {
            flags.push('P');
        }
        args.push(GrepArg::other(&flags));

        if self.untracked {
            args.push(GrepArg::other("--untracked"));
        }
        if self.no_index {
            args.push(GrepArg::other("--no-index"));
        }
        if self.no_recursive {
            args.push(GrepArg::other("--no-recursive"));
        }
        if matches!(mode, Mode::Parsing) {
            args.push(GrepArg::other("--heading"));
            args.push(GrepArg::other("-C"));
            args.push(GrepArg::other(&self.context_lines.0.to_string()));
        }
        if matches!(mode, Mode::Highlight) {
            args.push(GrepArg::other("-o"));
            args.push(GrepArg::other("--heading"));
        }

        if self.not_pattern.is_enabled(focus) || self.and_pattern.is_enabled(focus) {
            args.push(GrepArg::other("-e").line_breakable());
            args.push(self.pattern.clone());
        } else {
            args.push(self.pattern.clone().line_breakable());
        }

        if self.and_pattern.is_enabled(focus) {
            args.push(GrepArg::other("--and").line_breakable());
            args.push(GrepArg::other("-e"));
            args.push(self.and_pattern.clone());
        }
        if self.not_pattern.is_enabled(focus) {
            args.push(GrepArg::other("--and").line_breakable());
            args.push(GrepArg::other("--not"));
            args.push(GrepArg::other("-e"));
            args.push(self.not_pattern.clone());
        }
        if self.revision.is_enabled(focus) {
            args.push(self.revision.clone().line_breakable());
            if !self.path.is_enabled(focus) {
                args.push(GrepArg::other("--"));
            }
        }
        if self.path.is_enabled(focus) {
            args.push(GrepArg::other("--").line_breakable());
            args.push(self.path.clone());
        }
        args
    }
}

pub fn is_available() -> bool {
    // Check if `git` is accessible and we are within a Git directory.
    call(&["rev-parse", "--is-inside-work-tree"], true)
        .ok()
        .filter(|s| s.trim() == "true")
        .is_some()
}

fn call(args: &[&str], check_status: bool) -> orfail::Result<String> {
    let mut command = Command::new("git");
    let output = command
        .args(args)
        .output()
        .or_fail_with(|e| format!("Failed to execute `$ {command:?}`: {e}"))?;

    let error = |()| {
        format!(
            "Failed to execute `$ {command:?}`:\n{}\n",
            String::from_utf8_lossy(&output.stderr)
        )
    };
    (!check_status || output.status.success()).or_fail_with(error)?;
    (check_status || output.stderr.is_empty()).or_fail_with(error)?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_search_result() -> orfail::Result<()> {
        let output = r#"src/canvas.rs
315:        line.draw_token(2, Token::new("foo"));
316:        assert_eq!(line.text(), "  foo");
"#;
        let result = SearchResult::parse(output, Highlight::default()).or_fail()?;
        assert_eq!(result.files.len(), 1);

        let lines = result
            .files
            .get(&PathBuf::from("src/canvas.rs"))
            .or_fail()?;
        assert_eq!(lines.len(), 2);

        assert_eq!(lines[0].number.get(), 315);
        assert_eq!(
            lines[0].text,
            r#"        line.draw_token(2, Token::new("foo"));"#
        );

        assert_eq!(lines[1].number.get(), 316);
        assert_eq!(
            lines[1].text,
            r#"        assert_eq!(line.text(), "  foo");"#
        );

        Ok(())
    }

    #[test]
    fn parse_highlight() -> orfail::Result<()> {
        let output = r#"src/canvas.rs
315:40:foo
316:36:foo
src/git.rs
151:44:foo
152:40:foo
166:55:foo
172:51:foo"#;
        let highlight = Highlight::parse(output).or_fail()?;
        assert_eq!(highlight.lines.len(), 2);

        Ok(())
    }
}
