mod accept;
mod classify;
mod classify_expand;
mod classify_grade;
mod classify_import;
mod classify_journey;
mod cli;
mod decisions;
mod dispatch;
mod doctor_strict;
mod enrich;
mod export_repair;
mod heal;
mod help;
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
