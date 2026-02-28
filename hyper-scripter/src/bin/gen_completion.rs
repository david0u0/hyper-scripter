use clap::CommandFactory;
use hyper_scripter::args::Root;
use std::io::{stdout, Write};
use supplement::{generate, Config};

fn my_generate(w: &mut impl Write) {
    let mut cmd = Root::command();
    let config = Config::new().ignore(&["dump_args"]).ignore(&["load-utils"]);
    writeln!(w, "#![cfg_attr(rustfmt, rustfmt_skip)]").unwrap();
    generate(&mut cmd, config, w).unwrap();
}

fn main() {
    my_generate(&mut stdout());
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gen_in_sync() {
        let mut gen_content: Vec<u8> = vec![];
        my_generate(&mut gen_content);
        let gen_content = String::from_utf8(gen_content).unwrap();
        let file_content = include_str!("main/def.rs");

        assert_eq!(gen_content, file_content);
    }
}
