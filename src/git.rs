use std::{collections::BTreeMap, num::NonZeroUsize, path::PathBuf, process::Command};

use orfail::OrFail;

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

#[derive(Debug, Default, Clone)]
pub struct GrepOptions {
    pub pattern: String,
    pub and_pattern: String,
    pub not_pattern: String,
    pub revision: String,
    pub path: String,
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

impl GrepOptions {
    pub fn args(&self) -> Vec<String> {
        self.build_grep_args(Mode::External)
    }

    pub fn command_string(&self) -> String {
        // TODO: remove "$ "
        format!("$ git {}", self.build_grep_args(Mode::External).join(" "))
    }

    pub fn call(&self) -> orfail::Result<SearchResult> {
        // TODO: Execute in parallel.
        let args = self.build_grep_args(Mode::Highlight);
        let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        let output = call(&args, false).or_fail()?;
        let highlight = Highlight::parse(&output).or_fail()?;

        let args = self.build_grep_args(Mode::Parsing);
        let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        let output = call(&args, false).or_fail()?;

        SearchResult::parse(&output, highlight, self.context_lines.0).or_fail()
    }

    fn build_grep_args(&self, mode: Mode) -> Vec<String> {
        let mut args = vec!["grep".to_string(), "-nI".to_string()];
        if self.ignore_case {
            args.last_mut().expect("infallible").push('i');
        }
        if self.word_regexp {
            args.last_mut().expect("infallible").push('w');
        }
        if self.extended_regexp {
            args.last_mut().expect("infallible").push('E');
        }
        if self.fixed_strings {
            args.last_mut().expect("infallible").push('F');
        }
        if self.perl_regexp {
            args.last_mut().expect("infallible").push('P');
        }
        if self.untracked {
            args.push("--untracked".to_string());
        }
        if self.no_index {
            args.push("--no-index".to_string());
        }
        if self.no_recursive {
            args.push("--no-recursive".to_string());
        }
        if matches!(mode, Mode::Parsing) && self.context_lines.0 > 0 {
            args.push("--heading".to_string());
            args.push("-C".to_string());
            args.push(self.context_lines.0.to_string());
        }
        if matches!(mode, Mode::Highlight) {
            args.push("-o".to_string());
            args.push("--heading".to_string());
        }

        if !self.not_pattern.is_empty() || !self.and_pattern.is_empty() {
            args.push("-e".to_string());
        }
        args.push(self.pattern.clone());

        if !self.and_pattern.is_empty() {
            args.push("--and".to_string());
            args.push("-e".to_string());
            args.push(self.and_pattern.clone());
        }
        if !self.not_pattern.is_empty() {
            args.push("--and".to_string());
            args.push("--not".to_string());
            args.push("-e".to_string());
            args.push(self.not_pattern.clone());
        }
        if !self.revision.is_empty() {
            args.push(self.revision.clone());
            if self.path.is_empty() {
                args.push("--".to_string());
            }
        }
        if !self.path.is_empty() {
            args.push("--".to_string());
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
    let output = Command::new("git")
        .args(args)
        .output()
        .or_fail_with(|e| format!("Failed to execute `$ git {}`: {e}", args.join(" ")))?;

    let error = |()| {
        format!(
            "Failed to execute `$ git {}`:\n{}\n",
            args.join(" "),
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
