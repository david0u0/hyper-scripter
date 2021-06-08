use hyper_scripter::main_inner::main_with_args;

#[tokio::main]
async fn main() {
    env_logger::init();
    let args: Vec<_> = std::env::args().collect();
    let errs = match main_with_args(&args).await {
        Err(e) => vec![e],
        Ok(v) => v,
    };
    for err in errs.iter() {
        eprint!("{}", err);
    }
    if !errs.is_empty() {
        std::process::exit(1);
    }
}
