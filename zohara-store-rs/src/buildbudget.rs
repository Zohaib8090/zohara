//! How much of the computer a build from the AUR may use. Compiling a big program (a Rust or C++ app) can take many
//! gigabytes, and on a small computer that used to freeze everything or get the Store itself stopped by the system.
//! The Store works out a budget from the memory and cores the computer has, runs the build in its own memory group
//! (`systemd-run --user --scope`), so only the build can be stopped, and gives the build tools a matching number of jobs.

use std::process::Command;
use std::sync::OnceLock;

const GIB: u64 = 1 << 30;
/// What one compile job needs, roughly. Rust and C++ jobs often take 1 to 2 GB each.
const PER_JOB: u64 = GIB * 3 / 2;
/// Left to the rest of the system however big the budget is.
const HEADROOM: u64 = GIB;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub total_bytes: u64,
    pub swap_bytes: u64,
    /// The most memory the build may use.
    pub budget_bytes: u64,
    /// Parallel jobs for make, cargo and the like.
    pub jobs: u32,
    pub cores: u32,
}

/// Pure: half the memory (4 GB gives 2, 16 GB gives 8, 32 GB gives 16, 128 GB gives 64), but never more than what is
/// free now minus one GB for the system, never less than 1 GB, never more than three quarters of all of it. Jobs follow
/// from the budget (about 1.5 GB each) and the number of cores.
pub fn plan(total_kib: u64, available_kib: u64, swap_kib: u64, cores: u32) -> Plan {
    let total = total_kib * 1024;
    let avail = if available_kib == 0 { total } else { available_kib * 1024 };
    let mut budget = total / 2;
    budget = budget.min(avail.saturating_sub(HEADROOM));
    budget = budget.max(GIB).min(total / 4 * 3).max(GIB.min(total));
    let cores = cores.max(1);
    let jobs = ((budget / PER_JOB) as u32).clamp(1, cores);
    Plan { total_bytes: total, swap_bytes: swap_kib * 1024, budget_bytes: budget, jobs, cores }
}

fn field(meminfo: &str, name: &str) -> u64 {
    meminfo.lines().find_map(|l| l.strip_prefix(name)?.strip_prefix(':')?.split_whitespace().next()?.parse().ok()).unwrap_or(0)
}

/// The plan for this computer right now.
pub fn current() -> Plan {
    let m = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let cores = std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1);
    plan(field(&m, "MemTotal"), field(&m, "MemAvailable"), field(&m, "SwapTotal"), cores)
}

fn gb(bytes: u64) -> String {
    let g = bytes as f64 / GIB as f64;
    if g >= 10.0 { format!("{g:.0} GB") } else { format!("{g:.1} GB") }
}

/// For the warning before a build: what it may use.
pub fn describe(p: &Plan) -> String {
    format!("The Store limits a build to about {} of this computer's {} of memory and {} of its {} processor cores, so the rest keeps working.", gb(p.budget_bytes), gb(p.total_bytes), p.jobs, p.cores)
}

/// Said when a build ran out of memory.
pub fn too_big(p: &Plan) -> String {
    let swap = if p.swap_bytes == 0 { " This computer has no swap; adding some in Settings gives builds room to breathe." } else { "" };
    format!("The build needed more memory than the Store allows for it ({}), so it was stopped and nothing was changed. Closing other apps first can help.{swap}", gb(p.budget_bytes))
}

/// Whether the output of a failed build looks like it was stopped for lack of memory.
pub fn looks_out_of_memory(stderr: &str) -> bool {
    let e = stderr.to_lowercase();
    e.contains("signal: 9") || e.contains("sigkill") || e.contains("out of memory") || e.contains("cannot allocate memory") || e.contains("std::bad_alloc") || e.contains("terminated program cc1")
}

/// Whether the system lets this person limit a group of programs (`systemd-run --user --scope`). Asked once.
pub fn scopes_work() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        Command::new("systemd-run")
            .args(["--user", "--scope", "--quiet", "-p", "MemoryMax=1G", "true"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Wraps `inner` so it runs with the plan: in its own limited group when the system allows it, at low priority, with the
/// job counts set for the tools it starts.
pub fn wrap(inner: Command, p: &Plan, scope: bool) -> Command {
    let jobs = p.jobs.to_string();
    let mut c = if scope {
        let mut c = Command::new("systemd-run");
        c.args(["--user", "--scope", "--quiet", "--collect", "-p", &format!("MemoryMax={}", p.budget_bytes), "-p", "CPUWeight=50", "--", "nice", "-n", "10"]);
        c
    } else {
        let mut c = Command::new("nice");
        c.args(["-n", "10"]);
        c
    };
    c.arg(inner.get_program()).args(inner.get_args());
    c.env("MAKEFLAGS", format!("-j{jobs}")).env("CARGO_BUILD_JOBS", &jobs).env("NINJAFLAGS", format!("-j{jobs}")).env("CMAKE_BUILD_PARALLEL_LEVEL", &jobs);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    const GB4: u64 = 4 * 1024 * 1024;

    #[test]
    fn the_budget_grows_with_the_memory_the_computer_has() {
        let g = |gb: u64, avail: u64| plan(gb * GB4 / 4, avail * GB4 / 4, 0, 12).budget_bytes / GIB;
        assert_eq!(g(4, 3), 2, "the minimum computer builds in 2 GB");
        assert_eq!(g(8, 7), 4);
        assert_eq!(g(16, 14), 8);
        assert_eq!(g(32, 30), 16);
        assert_eq!(g(128, 120), 64);
    }

    #[test]
    fn a_busy_computer_gets_a_smaller_budget_and_never_less_than_one_gb() {
        // 16 GB with only 4 GB free: 3 GB (one is left for the system).
        assert_eq!(plan(16 * 1024 * 1024, 4 * 1024 * 1024, 0, 12).budget_bytes, 3 * GIB);
        // 4 GB with 1 GB free: the floor.
        assert_eq!(plan(4 * 1024 * 1024, 1024 * 1024, 0, 4).budget_bytes, GIB);
    }

    #[test]
    fn jobs_follow_the_budget_and_the_cores() {
        assert_eq!(plan(4 * 1024 * 1024, 3 * 1024 * 1024, 0, 4).jobs, 1);
        assert_eq!(plan(16 * 1024 * 1024, 14 * 1024 * 1024, 0, 12).jobs, 5);
        assert_eq!(plan(128 * 1024 * 1024, 120 * 1024 * 1024, 0, 8).jobs, 8, "never more than the cores");
        assert_eq!(plan(16 * 1024 * 1024, 14 * 1024 * 1024, 0, 0).jobs, 1);
    }

    #[test]
    fn unreadable_memory_figures_still_give_a_usable_plan() {
        let p = plan(0, 0, 0, 1);
        assert!(p.jobs == 1);
    }

    #[test]
    fn meminfo_is_read() {
        let m = "MemTotal:       16000000 kB\nMemAvailable:    9000000 kB\nSwapTotal:       8000000 kB\nSwapCached: 5 kB\n";
        assert_eq!((field(m, "MemTotal"), field(m, "MemAvailable"), field(m, "SwapTotal"), field(m, "Swap")), (16000000, 9000000, 8000000, 0));
    }

    #[test]
    fn wrapping_keeps_the_command_and_sets_the_limits() {
        let mut inner = Command::new("yay");
        inner.args(["-S", "--", "app"]);
        let p = plan(16 * 1024 * 1024, 14 * 1024 * 1024, 0, 12);
        let c = wrap(inner, &p, true);
        let args: Vec<String> = c.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        assert_eq!(c.get_program(), "systemd-run");
        assert!(args.contains(&format!("MemoryMax={}", p.budget_bytes)));
        assert_eq!(&args[args.len() - 4..], ["yay", "-S", "--", "app"]);
        let env: Vec<_> = c.get_envs().filter_map(|(k, v)| Some((k.to_string_lossy().to_string(), v?.to_string_lossy().to_string()))).collect();
        assert!(env.contains(&("MAKEFLAGS".into(), "-j5".into())) && env.contains(&("CARGO_BUILD_JOBS".into(), "5".into())));
        let plain = wrap(Command::new("yay"), &p, false);
        assert_eq!(plain.get_program(), "nice");
    }

    #[test]
    fn running_out_of_memory_is_recognised() {
        assert!(looks_out_of_memory("error: could not compile `x` (signal: 9, SIGKILL: kill)"));
        assert!(looks_out_of_memory("c++: fatal error: Killed signal terminated program cc1plus\ncompilation terminated."));
        assert!(!looks_out_of_memory("error: target not found: foo"));
    }

    #[test]
    #[ignore = "starts a real systemd scope"]
    fn live_a_wrapped_command_really_gets_the_limit_the_jobs_and_low_priority() {
        let plan = plan(16 * 1024 * 1024, 14 * 1024 * 1024, 0, 12);
        let mut inner = Command::new("sh");
        inner.args(["-c", "p=$(cut -d: -f3 /proc/self/cgroup); echo $(cat /sys/fs/cgroup$p/memory.max) $MAKEFLAGS $CARGO_BUILD_JOBS $(cut -d' ' -f19 /proc/self/stat)"]);
        let out = wrap(inner, &plan, true).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert_eq!(text, format!("{} -j5 5 10", plan.budget_bytes), "{text}");
    }
}
