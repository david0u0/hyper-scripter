use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use hyper_scripter::args::Root;
use std::io;

fn main() {
    let mut cmd = Root::command();
    generate(Shell::Fish, &mut cmd, "hs", &mut io::stdout());
}
