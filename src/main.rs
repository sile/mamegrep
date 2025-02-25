use clap::Parser;
use mamegrep::{
    app::App,
    git::{self, GrepOptions},
};

use orfail::OrFail;

/// Git grep TUI frontend.
#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Search pattern.
    pattern: Option<String>,

    /// `--and` search pattern.
    #[clap(short, long)]
    and_pattern: Option<String>,

    /// `--not` search pattern.
    #[clap(short, long)]
    not_pattern: Option<String>,

    /// Revision.
    #[clap(short, long)]
    revision: Option<String>,

    /// Path.
    #[clap(short, long)]
    path: Option<String>,
}

impl Args {
    fn into_grep_options(self) -> GrepOptions {
        let mut options = GrepOptions::default();
        options.pattern.text = self.pattern.unwrap_or_default();
        options.and_pattern.text = self.and_pattern.unwrap_or_default();
        options.not_pattern.text = self.not_pattern.unwrap_or_default();
        options.revision.text = self.revision.unwrap_or_default();
        options.path.text = self.path.unwrap_or_default();
        options
    }
}

fn main() -> orfail::Result<()> {
    let args = Args::parse();

    if !git::is_available() {
        eprintln!("error: no `git` command found, or not a Git directory");
        std::process::exit(1);
    };

    let app = App::new(args.into_grep_options()).or_fail()?;
    app.run().or_fail()?;

    Ok(())
}
