use std::process::Command;

use orfail::OrFail;

#[derive(Debug)]
enum Mode {
    External,
    Parsing,
    Highlight,
}

#[derive(Debug, Clone)]
pub struct SearchResult {}

impl SearchResult {
    fn parse(s: &str) -> orfail::Result<Self> {
        Ok(Self {})
    }
}

#[derive(Debug, Default, Clone)]
pub struct GrepOptions {
    pub pattern: String,
    pub before_context: usize,
    pub after_context: usize,
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

    pub fn call(&self) -> orfail::Result<String> {
        // TODO: no-hit handling
        //let args = self.build_grep_args(Mode::Highlight);
        let args = self.build_grep_args(Mode::Parsing);
        let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        let output = call(&args).or_fail()?;
        SearchResult::parse(&output).or_fail()?;
        Ok(output)
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
    call(&["rev-parse", "--is-inside-work-tree"])
        .ok()
        .filter(|s| s.trim() == "true")
        .is_some()
}

fn call(args: &[&str]) -> orfail::Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .or_fail_with(|e| format!("Failed to execute `$ git {}`: {e}", args.join(" ")))?;

    output.status.success().or_fail_with(|()| {
        format!(
            "Failed to execute `$ git {}`:\n{}\n",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
    })?;

    String::from_utf8(output.stdout).or_fail()
}
