/// Institutional CPU core pinning and thread isolation utilities.
///
/// Prevents cache line bouncing and OS thread migration in high-frequency trading loops.
/// Uses the Linux `sched_setaffinity(2)` ABI directly — zero new dependencies added to Cargo.toml.

#[cfg(target_os = "linux")]
pub fn pin_current_thread_to_core(core_id: usize) -> bool {
    const BITS_PER_USIZE: usize = 8 * std::mem::size_of::<usize>();
    const CPU_SETSIZE: usize = 1024;
    const NUM_WORDS: usize = CPU_SETSIZE / BITS_PER_USIZE;

    #[repr(C)]
    struct CpuSet {
        bits: [usize; NUM_WORDS],
    }

    extern "C" {
        fn sched_setaffinity(pid: i32, cpusetsize: usize, mask: *const CpuSet) -> i32;
    }

    let idx = core_id / BITS_PER_USIZE;
    let offset = core_id % BITS_PER_USIZE;

    if idx >= NUM_WORDS {
        return false;
    }

    let mut set = CpuSet {
        bits: [0; NUM_WORDS],
    };
    set.bits[idx] |= 1 << offset;

    let res = unsafe { sched_setaffinity(0, std::mem::size_of::<CpuSet>(), &set) };
    res == 0
}

#[cfg(not(target_os = "linux"))]
pub fn pin_current_thread_to_core(_core_id: usize) -> bool {
    // Graceful no-op fallback on macOS / Windows
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_current_thread() {
        let ok = pin_current_thread_to_core(0);
        #[cfg(target_os = "linux")]
        assert!(ok, "Core 0 pinning should succeed on Linux");
    }
}
