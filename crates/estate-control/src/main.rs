mod accept;
mod classify;
mod classify_expand;
mod classify_grade;
mod classify_import;
mod classify_journey;
mod cli;
mod cohesion_prove;
mod control_plane_prove;
mod crew_session_prove;
mod decision_prove;
mod decisions;
mod dispatch;
mod doctor_strict;
mod dual_prove;
mod enrich;
mod export_plugin;
mod export_repair;
mod heal;
mod help;
mod helpers;
mod ops;
mod pack_mcp;
mod pack_session;
mod plan_apply;
mod plugin_install;
mod plugin_prove;
mod routine_runner;
mod runner_prove;
mod routines;
mod specialty_bind;
mod suggest;
mod watch;

fn main() {
    if let Err(err) = dispatch::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
