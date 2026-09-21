mod accept;
mod cli;
mod dispatch;
mod doctor_strict;
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
