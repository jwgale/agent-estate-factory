use anyhow::Result;

mod accept;
mod cli;
mod dispatch;
mod heal;
mod helpers;
mod ops;
mod plan_apply;
mod suggest;
mod watch;

fn main() {
    if let Err(err) = dispatch::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
