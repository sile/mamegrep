use std::{collections::BTreeMap, num::NonZeroUsize, path::PathBuf, process::Command};

use orfail::OrFail;

#[derive(Debug)]
enum Mode {
    External,
    Parsing,
    Highlight,
}

#[derive(Debug, Default, Clone)]
pub struct Highlight {
    pub lines: BTreeMap<(PathBuf, NonZeroUsize), Vec<MatchLineColumn>>,
}

impl Highlight {
    fn parse(s: &str) -> orfail::Result<Self> {
        let mut lines = BTreeMap::<_, Vec<_>>::new();
        let mut current = PathBuf::new();
        for line in s.lines() {
            if let Some(m) = MatchLineColumn::parse(line) {
                lines
                    .entry((current.clone(), m.line_number))
                    .or_default()
                    .push(m);
            } else {
                current = PathBuf::from(line);
            }
        }
        Ok(Self { lines })
    }
}

#[derive(Debug, Default, Clone)]
pub struct SearchResult {
    pub files: BTreeMap<PathBuf, Vec<MatchLine>>,
    pub max_line_width: usize,
    // TODO: highlight
}

impl SearchResult {
    fn parse(s: &str) -> orfail::Result<Self> {
        let mut files = BTreeMap::<_, Vec<_>>::new();
        let mut current = PathBuf::new();
        let mut max_line_width = 1;
        for line in s.lines() {
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
        })
    }
}

#[derive(Debug, Clone)]
pub struct MatchLine {
    pub number: NonZeroUsize,
    pub text: String,
}

impl MatchLine {
    fn parse(line: &str) -> Option<Self> {
        let i = line.find(':')?;
        let number = line[..i].parse().ok()?;
        Some(Self {
            number,
            text: line[i + 1..].to_owned(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MatchLineColumn {
    pub line_number: NonZeroUsize,
    pub column_offset: usize,
    pub text_chars: usize,
}

impl MatchLineColumn {
    fn parse(line: &str) -> Option<Self> {
        let i = line.find(':')?;
        let line_number = line[..i].parse().ok()?;

        let line = &line[i + 1..];
        let i = line.find(':')?;
        let column_offset = line[..i].parse().ok()?;
        Some(Self {
            line_number,
            column_offset,
            text_chars: line[i + 1..].chars().count(),
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct GrepOptions {
    pub pattern: String,
    pub before_context: usize, // TODO: delete (used for detailed page)
    pub after_context: usize,  // TODO: delete (ditto)
    pub ignore_case: bool,
    // TODO:
    // --no-index
    // --untracked
    // --no-recursive
    // -w (--word-regex)
    // -E (--extended-regexp)
    // -F (--fixed-strings)
    // -P (--perl-regexp)
    // -e, --and, --or, --not, (, )
    // --all-match
    // <rev>
    // -- <path> (for internal to expand the matched context)
}

impl GrepOptions {
    pub fn command_string(&self) -> String {
        // TODO: remove "$ "
        format!("$ git {}", self.build_grep_args(Mode::External).join(" "))
    }

    pub fn call(&self) -> orfail::Result<SearchResult> {
        //let args = self.build_grep_args(Mode::Highlight);
        let args = self.build_grep_args(Mode::Parsing);
        let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        let output = call(&args, false).or_fail()?;
        SearchResult::parse(&output).or_fail()
    }

    fn build_grep_args(&self, mode: Mode) -> Vec<String> {
        let mut args = vec!["grep".to_string(), "-nI".to_string()];
        if self.ignore_case {
            args.last_mut().expect("infallible").push('i');
        }
        if matches!(mode, Mode::Parsing) {
            args.push("--heading".to_string());
        }
        if matches!(mode, Mode::Highlight) {
            args.push("-o".to_string());
            args.push("--column".to_string());
        }
        if self.before_context > 0 {
            args.push("-B".to_string());
            args.push(self.before_context.to_string());
        }
        if self.after_context > 0 {
            args.push("-A".to_string());
            args.push(self.after_context.to_string());
        }
        args.push(self.pattern.clone());
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
        let result = SearchResult::parse(&output).or_fail()?;
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
        assert_eq!(highlight.lines.len(), 6);

        Ok(())
    }
}
