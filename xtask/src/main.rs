use clap::Parser;

fn main() {
    if let Err(e) = xtask::cmd::dispatch(xtask::cli::Cli::parse()) {
        eprintln!("\nerror: {e}");
        std::process::exit(1);
    }
}
