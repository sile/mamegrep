use std::process::Command;

use orfail::OrFail;

#[derive(Debug, Default, Clone)]
pub struct GrepOptions {
    pub pattern: Option<String>,
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
