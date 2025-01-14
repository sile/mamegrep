use mamegrep::{app::App, git};

use orfail::OrFail;

fn main() -> orfail::Result<()> {
    if !git::is_available() {
        eprintln!("error: no `git` command found, or not a Git directory");
        std::process::exit(1);
    };

    let app = App::new().or_fail()?;
    app.run().or_fail()?;

    Ok(())
}
