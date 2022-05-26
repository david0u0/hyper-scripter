use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use hyper_scripter::{args::Root, APP_NAME};
use std::io;

#[derive(Parser)]
struct Args {
    shell_type: Shell,
}

fn main() {
    let args = Args::parse();
    let mut cmd = Root::command();
    generate(args.shell_type, &mut cmd, APP_NAME, &mut io::stdout());
}
