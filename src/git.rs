use std::{borrow::Cow, collections::BTreeMap, num::NonZeroUsize, path::PathBuf, process::Command};

use orfail::OrFail;
use unicode_width::UnicodeWidthStr;

use crate::app::Focus;

pub const DEFAULT_CONTEXT_LINES: usize = 4;

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
            if let Some(m) = MatchLine::parse(line) {
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
    pub files: BTreeMap<PathBuf, Vec<MatchLine>>,
    pub max_line_width: usize,
    pub highlight: Highlight,
    pub context_lines: usize,
}

impl SearchResult {
    fn parse(s: &str, highlight: Highlight, context_lines: usize) -> orfail::Result<Self> {
        let mut files = BTreeMap::<_, Vec<_>>::new();
        let mut current = PathBuf::new();
        let mut max_line_width = 1;
        for line in s.lines() {
            if line == "--" {
                continue;
            }

            if let Some(line) = MatchLine::parse(line) {
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
            context_lines,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MatchLine {
    pub number: NonZeroUsize,
    pub text: String,
    pub matched: bool,
}

impl MatchLine {
    fn parse(line: &str) -> Option<Self> {
        for (i, c) in line.char_indices() {
            match c {
                ':' => {
                    let number = line[..i].parse().ok()?;
                    return Some(Self {
                        number,
                        text: line[i + 1..].to_owned(),
                        matched: true,
                    });
                }
                '-' => {
                    let number = line[..i].parse().ok()?;
                    return Some(Self {
                        number,
                        text: line[i + 1..].to_owned(),
                        matched: false,
                    });
                }
                '0'..='9' => {}
                _ => return None,
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct ContextLines(pub usize);

impl Default for ContextLines {
    fn default() -> Self {
        Self(DEFAULT_CONTEXT_LINES)
    }
}

#[derive(Debug, Clone)]
pub struct GrepArg {
    pub kind: GrepArgKind,
    pub text: String, // TODO: private
}

impl GrepArg {
    pub fn new(kind: GrepArgKind, text: &str) -> Self {
        Self {
            kind,
            text: text.to_string(),
        }
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
        self.text[..i].chars().rev().next()
    }

    // TODO: rename
    pub fn is_enabled(&self, focus: Focus) -> bool {
        !self.is_empty() || self.kind.is_focused(focus)
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn text(&self, focus: Focus) -> Cow<str> {
        match (self.kind, focus) {
            (GrepArgKind::Pattern, Focus::Pattern)
            | (GrepArgKind::AndPattern, Focus::AndPattern)
            | (GrepArgKind::NotPattern, Focus::NotPattern)
            | (GrepArgKind::Path, Focus::Path)
            | (GrepArgKind::Revision, Focus::Revision)
            | (GrepArgKind::Other, _) => Cow::Borrowed(&self.text),
            _ => self.to_quoted_text(),
        }
    }

    pub fn width(&self, focus: Focus) -> usize {
        self.text(focus).width()
    }

    pub fn to_quoted_text(&self) -> Cow<str> {
        if self.text.is_empty() {
            return Cow::Borrowed("''");
        } else if !self.text.contains([' ', '\'']) {
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
        match (self, focus) {
            (Self::Pattern, Focus::Pattern)
            | (Self::AndPattern, Focus::AndPattern)
            | (Self::NotPattern, Focus::NotPattern)
            | (Self::Revision, Focus::Revision)
            | (Self::Path, Focus::Path) => true,
            _ => false,
        }
    }
}

// TODO: move
impl GrepArg {
    fn other(s: &str) -> Self {
        Self::new(GrepArgKind::Other, s)
    }

    fn pattern(s: &str) -> Self {
        Self::new(GrepArgKind::Pattern, s)
    }

    fn and_pattern(s: &str) -> Self {
        Self::new(GrepArgKind::AndPattern, s)
    }

    fn not_pattern(s: &str) -> Self {
        Self::new(GrepArgKind::NotPattern, s)
    }

    fn revision(s: &str) -> Self {
        Self::new(GrepArgKind::Revision, s)
    }

    fn path(s: &str) -> Self {
        Self::new(GrepArgKind::Path, s)
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
            pattern: GrepArg::pattern(""),
            and_pattern: GrepArg::and_pattern(""),
            not_pattern: GrepArg::not_pattern(""),
            revision: GrepArg::revision(""),
            path: GrepArg::path(""),
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

    pub fn command_string(&self) -> String {
        // TODO: remove "$ "
        format!(
            "$ git {}",
            self.build_grep_args(Mode::External, Focus::SearchResult)
                .into_iter()
                .map(|x| x.to_quoted_text().into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }

    pub fn call(&self) -> orfail::Result<SearchResult> {
        // TODO: Execute in parallel.
        let args = self.build_grep_args(Mode::Highlight, Focus::SearchResult);
        let args = args.iter().map(|s| s.to_quoted_text()).collect::<Vec<_>>();
        let output = call(&args, false).or_fail()?;
        let highlight = Highlight::parse(&output).or_fail()?;

        let args = self.build_grep_args(Mode::Parsing, Focus::SearchResult);
        let args = args.iter().map(|s| s.to_quoted_text()).collect::<Vec<_>>();
        let output = call(&args, false).or_fail()?;

        SearchResult::parse(&output, highlight, self.context_lines.0).or_fail()
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
        if matches!(mode, Mode::Parsing) && self.context_lines.0 > 0 {
            args.push(GrepArg::other("--heading"));
            args.push(GrepArg::other("-C"));
            args.push(GrepArg::other(&self.context_lines.0.to_string()));
        }
        if matches!(mode, Mode::Highlight) {
            args.push(GrepArg::other("-o"));
            args.push(GrepArg::other("--heading"));
        }

        if self.not_pattern.is_enabled(focus) || self.and_pattern.is_enabled(focus) {
            args.push(GrepArg::other("-e"));
        }
        args.push(self.pattern.clone());

        if self.and_pattern.is_enabled(focus) {
            args.push(GrepArg::other("--and"));
            args.push(GrepArg::other("-e"));
            args.push(self.and_pattern.clone());
        }
        if self.not_pattern.is_enabled(focus) {
            args.push(GrepArg::other("--and"));
            args.push(GrepArg::other("--not"));
            args.push(GrepArg::other("-e"));
            args.push(self.not_pattern.clone());
        }
        if self.revision.is_enabled(focus) {
            args.push(self.revision.clone());
            if !self.path.is_enabled(focus) {
                args.push(GrepArg::other("--"));
            }
        }
        if self.path.is_enabled(focus) {
            args.push(GrepArg::other("--"));
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

fn call<S>(args: &[S], check_status: bool) -> orfail::Result<String>
where
    S: AsRef<str>,
{
    let output = Command::new("git")
        .args(args.iter().map(|a| a.as_ref()))
        .output()
        .or_fail_with(|e| {
            format!(
                "Failed to execute `$ git {}`: {e}",
                args.iter()
                    .map(|a| a.as_ref())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })?;

    let error = |()| {
        format!(
            "Failed to execute `$ git {}`:\n{}\n",
            args.iter()
                .map(|a| a.as_ref())
                .collect::<Vec<_>>()
                .join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
    };
    (!check_status || output.status.success()).or_fail_with(error)?;
    (check_status || output.stderr.is_empty()).or_fail_with(error)?;

    String::from_utf8(output.stdout).or_fail()
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
        let result = SearchResult::parse(&output, Highlight::default()).or_fail()?;
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
        let highlight = Highlight::parse(&output).or_fail()?;
        assert_eq!(highlight.lines.len(), 2);

        Ok(())
    }
}
