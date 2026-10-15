//! Safe process signalling. Every group kill goes through here.
//!
//! Why: on some hosts `/usr/bin/kill -KILL -<pid>` (procps) parses the negative pid as an option and
//! issues `kill(-1, SIGKILL)`, which kills every process the user owns (seen 2026-10-14 21:19 CDT on
//! the 5090: GUI session, user@1000 and user-owned system services were SIGKILLed). We never shell out to
//! `kill` for signals. We call kill(2) directly, and we refuse any target that could widen the blast radius:
//! pid/pgid 0 or 1, anything that does not fit in a positive i32 (u32::MAX would wrap to -1), our own
//! process, and our own process group.

#[cfg(unix)]
extern "C" {
    #[link_name = "kill"]
    fn c_kill(pid: i32, sig: i32) -> i32;
    #[link_name = "getpgrp"]
    fn c_getpgrp() -> i32;
}

pub(crate) const SIGTERM: i32 = 15;
pub(crate) const SIGKILL: i32 = 9;

/// Positive i32 > 1, or None. Pure; never signals.
pub(crate) fn checked_id(id: u32) -> Option<i32> {
    if id <= 1 || id > i32::MAX as u32 {
        return None;
    }
    Some(id as i32)
}

/// A process group we may signal: positive, > 1, and not our own process or group. Pure apart from getpgrp().
pub(crate) fn checked_pgid(pgid: u32) -> Option<i32> {
    let g = checked_id(pgid)?;
    if g as u32 == std::process::id() {
        return None;
    }
    #[cfg(unix)]
    {
        if g == unsafe { c_getpgrp() } {
            return None;
        }
    }
    Some(g)
}

/// Signal the process group `pgid` (the child was spawned with `process_group(0)`, so its pgid == its pid).
/// Returns false without signalling when the target is refused.
pub(crate) fn kill_process_group(pgid: u32, sig: i32) -> bool {
    let Some(g) = checked_pgid(pgid) else {
        return false;
    };
    #[cfg(unix)]
    {
        debug_assert!(g > 1);
        unsafe { c_kill(-g, sig) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = (g, sig);
        false
    }
}

/// Signal one process by pid (never a group, never -1/0/1, never ourselves).
pub(crate) fn kill_pid(pid: u32, sig: i32) -> bool {
    let Some(p) = checked_id(pid) else {
        return false;
    };
    if p as u32 == std::process::id() {
        return false;
    }
    #[cfg(unix)]
    {
        unsafe { c_kill(p, sig) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = (p, sig);
        false
    }
}

/// Liveness probe (signal 0) for one pid. Refused targets read as not alive.
pub(crate) fn pid_alive(pid: u32) -> bool {
    let Some(p) = checked_id(pid) else {
        return false;
    };
    #[cfg(unix)]
    {
        unsafe { c_kill(p, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = p;
        false
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    #[test]
    fn guards_refuse_dangerous_targets_without_signalling() {
        // Pure checks only: nothing here reaches kill(2).
        for bad in [0u32, 1, u32::MAX, i32::MAX as u32 + 1, std::process::id()] {
            assert_eq!(checked_pgid(bad), None, "pgid {bad} must be refused");
        }
        let own = unsafe { c_getpgrp() } as u32;
        assert_eq!(checked_pgid(own), None, "own process group must be refused");
        assert_eq!(checked_id(0), None);
        assert_eq!(checked_id(1), None);
        assert_eq!(checked_id(u32::MAX), None);
        assert_eq!(checked_id(4242), Some(4242));
    }

    fn wait_gone(pid: u32, secs: u64) -> bool {
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(secs) {
            if !pid_alive(pid) {
                return true;
            }
            sleep(Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn group_kill_kills_only_the_child_group_and_a_sentinel_sibling_survives() {
        // Sentinel: a sibling in its OWN group. It must survive the group kill.
        let mut sentinel = Command::new("sleep")
            .arg("60")
            .process_group(0)
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn sentinel");
        // Target: a shell that forks two grandchildren in the same group, prints their pids, then waits.
        let mut target = Command::new("sh")
            .args(["-c", "sleep 60 & echo $!; sleep 60 & echo $!; wait"])
            .process_group(0)
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn target");
        let tpid = target.id();
        let mut out = target.stdout.take().unwrap();
        let mut buf = String::new();
        let t = Instant::now();
        while buf.lines().count() < 2 && t.elapsed() < Duration::from_secs(5) {
            use std::io::Read;
            let mut chunk = [0u8; 64];
            let n = out.read(&mut chunk).unwrap_or(0);
            buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
        let grandkids: Vec<u32> = buf.lines().filter_map(|l| l.trim().parse().ok()).collect();
        assert_eq!(grandkids.len(), 2, "grandchild pids: {buf:?}");
        assert!(checked_pgid(tpid).is_some());
        assert_ne!(tpid as i32, unsafe { c_getpgrp() });

        assert!(
            kill_process_group(tpid, SIGKILL),
            "group kill should succeed"
        );
        let status = target.wait().expect("reap target");
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(SIGKILL), "target died by SIGKILL");
        for g in &grandkids {
            assert!(wait_gone(*g, 5), "grandchild {g} survived the group kill");
        }
        // Blast radius: sentinel and this test process are still alive.
        assert!(
            pid_alive(sentinel.id()),
            "sentinel sibling was killed: group kill escaped its group"
        );
        assert!(sentinel.try_wait().unwrap().is_none(), "sentinel exited");
        assert!(kill_pid(sentinel.id(), SIGKILL));
        let _ = sentinel.wait();
    }
}
