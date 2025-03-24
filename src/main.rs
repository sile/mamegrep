use mamegrep::{
    app::App,
    git::{self, GrepOptions},
};

use orfail::OrFail;

fn main() -> noargs::Result<()> {
    let mut options = GrepOptions::default();

    let mut args = noargs::args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");
    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if noargs::HELP_FLAG.take(&mut args).is_present() {
        args.metadata_mut().help_mode = true;
    }
    options.and_pattern.text = noargs::opt("and-pattern")
        .short('a')
        .ty("PATTERN")
        .doc("`--and` search pattern")
        .take(&mut args)
        .parse_if_present()?
        .unwrap_or_default();
    options.not_pattern.text = noargs::opt("not-pattern")
        .short('n')
        .ty("PATTERN")
        .doc("`--not` search pattern")
        .take(&mut args)
        .parse_if_present()?
        .unwrap_or_default();
    options.revision.text = noargs::opt("revision")
        .short('r')
        .ty("REVISION")
        .doc("Revision")
        .take(&mut args)
        .parse_if_present()?
        .unwrap_or_default();
    options.path.text = noargs::opt("path")
        .short('p')
        .ty("PATH")
        .doc("Path")
        .take(&mut args)
        .parse_if_present()?
        .unwrap_or_default();
    options.pattern.text = noargs::arg("PATTERN")
        .doc("Search pattern")
        .take(&mut args)
        .parse_if_present()?
        .unwrap_or_default();
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    if !git::is_available() {
        eprintln!("error: no `git` command found, or not a Git directory");
        std::process::exit(1);
    };

    let app = App::new(options).or_fail()?;
    app.run().or_fail()?;

    Ok(())
}
