mamegrep
========

[![mamegrep](https://img.shields.io/crates/v/mamegrep.svg)](https://crates.io/crates/mamegrep)
[![Documentation](https://docs.rs/mamegrep/badge.svg)](https://docs.rs/mamegrep)
[![Actions Status](https://github.com/sile/mamegrep/workflows/CI/badge.svg)](https://github.com/sile/mamegrep/actions)
![License](https://img.shields.io/crates/l/mamegrep)

A TUI tool for `$ git grep` to easily edit search patterns and view results.

Installation
------------

### Pre-built binaries

Pre-built binaries for Linux and MacOS are available in [the releases page](https://github.com/sile/mamegrep/releases).

```console
// An example to download the binary for Linux.
$ VERSION=0.1.0
$ curl -L https://github.com/sile/mamegrep/releases/download/v${VERSION}/mamegrep-${VERSION}.x86_64-unknown-linux-musl -o mamegrep
$ chmod +x mamegrep
$ ./mamegrep -h
```

### With [Cargo](https://doc.rust-lang.org/cargo/)

If you have installed `cargo` (the package manager for Rust), you can install `mamegrep` with the following command:

```console
$ cargo install mamegrep
$ mamegrep -h
```

Please refine the following section:

Basic Usage
-----------

To use `mamegrep`, execute the command within a Git directory.
Once launched, key bindings will appear in the top-right corner of the terminal.

To perform a search, enter your search pattern and press the Enter key. 
If `mamegrep` exits, the equivalent `$ git grep` command used to generate the result will be displayed in the standard output.

You Might Also Be Interested In
-------------------------------

- [mamediff](https://github.com/sile/mamediff): A TUI tool for `$ git diff` and `$ git apply`
