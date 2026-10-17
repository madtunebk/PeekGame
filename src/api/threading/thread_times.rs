//! Real Linux task accounting converted to Windows FILETIME units.
use crate::api::system::{error, startup};
use std::ffi::c_void;
unsafe extern "C" {
    fn sysconf(name: i32) -> i64;
}
const UNITS: u128 = 10_000_000;
const EPOCH: u128 = 11_644_473_600;
fn parse(stat: &str, boot: u64, hz: u64) -> Option<[u64; 4]> {
    if hz == 0 {
        return None;
    }
    // comm can contain spaces and ')'; fields after its final ')' start at 3.
    let fields: Vec<_> = stat.rsplit_once(')')?.1.split_whitespace().collect();
    let number = |index: usize| fields.get(index)?.parse::<u64>().ok();
    let ticks = |n: u64| u64::try_from(n as u128 * UNITS / hz as u128).ok();
    let creation =
        u64::try_from((boot as u128 + EPOCH) * UNITS + number(19)? as u128 * UNITS / hz as u128)
            .ok()?;
    Some([creation, 0, ticks(number(12)?)?, ticks(number(11)?)?])
}
pub(super) fn snapshot(id: u32) -> Option<[u64; 4]> {
    let stat = std::fs::read_to_string(format!("/proc/self/task/{id}/stat")).ok()?;
    let boot = std::fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .parse()
        .ok()?;
    let hz = unsafe { sysconf(2) }; // _SC_CLK_TCK
    let mut times = parse(&stat, boot, u64::try_from(hz).ok()?)?;
    if id == startup::thread_id() {
        // Linux x86-64 struct rusage: two timeval fields and fourteen long counters.
        #[repr(C)]
        struct Usage {
            user: [i64; 2],
            kernel: [i64; 2],
            counters: [i64; 14],
        }
        unsafe extern "C" {
            fn getrusage(who: i32, usage: *mut Usage) -> i32;
        }
        let mut usage = Usage {
            user: [0; 2],
            kernel: [0; 2],
            counters: [0; 14],
        };
        if unsafe { getrusage(1, &mut usage) } == 0 {
            // RUSAGE_THREAD
            let duration =
                |t: [i64; 2]| u64::try_from(t[0] as i128 * 10_000_000 + t[1] as i128 * 10).ok();
            times[2] = duration(usage.kernel)?;
            times[3] = duration(usage.user)?;
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    times[1] =
        u64::try_from((now.as_secs() as u128 + EPOCH) * UNITS + now.subsec_nanos() as u128 / 100)
            .ok()?;
    Some(times)
}
/// Four required FILETIME outputs; their Windows alignment is only four bytes.
pub unsafe extern "win64" fn get(
    handle: usize,
    creation: *mut c_void,
    exit: *mut c_void,
    kernel: *mut c_void,
    user: *mut c_void,
) -> i32 {
    if super::threads::id_of(handle).is_none() {
        error::set(6);
        return 0;
    }
    let outputs = [creation, exit, kernel, user];
    if outputs.iter().any(|p| p.is_null()) {
        error::set(998);
        return 0;
    }
    let Some(times) = super::threads::accounting(handle) else {
        error::set(5);
        return 0;
    };
    for (out, value) in outputs.into_iter().zip(times) {
        unsafe { out.cast::<u64>().write_unaligned(value) };
    }
    1
}
#[cfg(test)]
mod tests {
    #[test]
    fn proc_fields() {
        let mut fields = vec!["0"; 20];
        fields[0] = "R";
        fields[11] = "25";
        fields[12] = "7";
        fields[19] = "123";
        let line = format!("10 (name with ) spaces) {}", fields.join(" "));
        assert_eq!(
            super::parse(&line, 1000, 100),
            Some([116444746012300000, 0, 700000, 2500000])
        );
        assert!(super::parse(&line, 1000, 0).is_none());
        assert!(super::parse("bad", 1000, 100).is_none());
    }
}
