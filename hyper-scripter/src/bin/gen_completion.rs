use hyper_scripter::args::Root;
use structopt::clap::Shell;
use structopt::StructOpt;

fn main() {
    let mut clap = Root::clap();
    clap.gen_completions("hs", Shell::Fish, ".")
}
