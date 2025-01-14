use mamegrep::git;

fn main() -> orfail::Result<()> {
    if !git::is_available() {
        eprintln!("error: no `git` command found, or not a Git directory");
        std::process::exit(1);
    };

    Ok(())
}
