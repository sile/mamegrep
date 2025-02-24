use crossterm::style::{Attribute, Attributes, ContentStyle, PrintStyledContent, StyledContent};
use mamegrep::{app::App, git};

use orfail::OrFail;

fn main() -> orfail::Result<()> {
    let args = Args::parse();

    if !git::is_available() {
        eprintln!("error: no `git` command found, or not a Git directory");
        std::process::exit(1);
    };

    let app = App::new(args.pattern).or_fail()?;
    app.run().or_fail()?;

    Ok(())
}

#[derive(Default)]
struct Args {
    pattern: Option<String>,
}

impl Args {
    fn parse() -> Self {
        let mut args = Args::default();

        if std::env::args().count() < 2 {
            return args;
        }

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "-h" | "--help" => {
                    println!("Git grep TUI frontend");
                    println!();
                    println!(
                        "{} {} [OPTIONS] [PATTERN]",
                        bold_underline("Usage:"),
                        bold("mamegrep"),
                    );
                    println!();
                    println!("{}", bold_underline("Pattern:"));
                    println!("  Initial search pattern");
                    println!();
                    println!("{}", bold_underline("Options:"));
                    println!(" {}  Print help", bold(" -h, --help"));
                    println!(" {}   Print version", bold(" --version"));

                    std::process::exit(0);
                }
                "--version" => {
                    println!("mamegrep {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                pattern if args.pattern.is_none() => {
                    args.pattern = Some(pattern.to_owned());
                }
                _ => {
                    eprintln!("{} unexpected argment '{arg}' found", bold("error:"),);
                    eprintln!();
                    eprintln!(
                        "{} {} [OPTIONS] [PATTERN]",
                        bold_underline("Usage:"),
                        bold("mamegrep"),
                    );
                    eprintln!();
                    eprintln!("For more information, try '--help'.");

                    std::process::exit(1);
                }
            }
        }

        args
    }
}

fn bold(s: &str) -> PrintStyledContent<&str> {
    let content = StyledContent::new(
        ContentStyle {
            attributes: Attributes::default().with(Attribute::Bold),
            ..Default::default()
        },
        s,
    );
    PrintStyledContent(content)
}

fn bold_underline(s: &str) -> PrintStyledContent<&str> {
    let content = StyledContent::new(
        ContentStyle {
            attributes: Attributes::default()
                .with(Attribute::Bold)
                .with(Attribute::Underlined),
            ..Default::default()
        },
        s,
    );
    PrintStyledContent(content)
}
