use clap::CommandFactory;
use hyper_scripter::args::Root;
use std::io::{stdout, Write};
use supplement::{generate, Config};

fn ignore_flag(conf: Config, cmd: &[&str], flag: &str) -> Config {
    let mut arr = cmd.to_vec();
    arr.push(flag);
    conf.ignore(&arr)
}

fn ignore_event_flags(mut conf: Config, cmd: &[&str]) -> Config {
    conf = ignore_flag(conf, cmd, "no_trace");
    conf = ignore_flag(conf, cmd, "humble");
    conf
}
fn ignore_global_flags(mut conf: Config, cmd: &[&str]) -> Config {
    conf = ignore_flag(conf, cmd, "archaeology");
    conf = ignore_flag(conf, cmd, "select");
    conf = ignore_flag(conf, cmd, "all");
    conf = ignore_flag(conf, cmd, "recent");
    conf = ignore_flag(conf, cmd, "timeless");
    ignore_event_flags(conf, cmd)
}

fn my_generate(w: &mut impl Write) {
    let mut cmd = Root::command();
    let mut config = Config::new().ignore(&["dump_args"]).ignore(&["load-utils"]);

    config = ignore_global_flags(config, &["types"]);
    config = ignore_global_flags(config, &["tags"]);
    config = ignore_global_flags(config, &["tags", "set"]);
    config = ignore_global_flags(config, &["tags", "unset"]);
    config = ignore_global_flags(config, &["tags", "toggle"]);

    config = ignore_event_flags(config, &["ls"]);

    // NOTE: 還有一些東西理論上可以忽略，如 `history show --humble`，但太麻煩了
    // 這裡只忽略常用且會被太多參數影響者

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
