//! Aggregate CPU accounting; kernel time includes idle time as on Windows.
use super::error;
unsafe extern "C" {
    fn sysconf(name: i32) -> i64;
}
fn parse(text: &str, hz: u64) -> Option<[u64; 3]> {
    if hz == 0 {
        return None;
    }
    let mut fields = text.lines().next()?.split_whitespace();
    if fields.next()? != "cpu" {
        return None;
    }
    let values = fields
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if values.len() < 4 {
        return None;
    }
    let at = |i| values.get(i).copied().unwrap_or(0) as u128;
    // Linux guest counters are already included in user/nice; do not add them twice.
    let idle = at(3) + at(4);
    let kernel = idle + at(2) + at(5) + at(6);
    let user = at(0) + at(1);
    let convert = |ticks: u128| u64::try_from(ticks * 10_000_000 / hz as u128).ok();
    Some([convert(idle)?, convert(kernel)?, convert(user)?])
}
/// FILETIME is two DWORDs (4-byte aligned), and each output is optional.
pub unsafe extern "win64" fn get(idle: *mut u64, kernel: *mut u64, user: *mut u64) -> i32 {
    let hz = unsafe { sysconf(2) }; // Linux _SC_CLK_TCK
    let times = std::fs::read_to_string("/proc/stat")
        .ok()
        .and_then(|text| parse(&text, hz.max(0) as u64));
    let Some(times) = times else {
        error::set(31); // ERROR_GEN_FAILURE; no invented counters on read/parse failure.
        return 0;
    };
    for (out, value) in [idle, kernel, user].into_iter().zip(times) {
        if !out.is_null() {
            unsafe { out.write_unaligned(value) };
        }
    }
    1
}
#[cfg(test)]
mod tests {
    use super::parse;
    #[test]
    fn cpu_accounting() {
        assert_eq!(
            parse("cpu 100 20 30 400 5 6 7 8 90 10\ncpu0 0", 100),
            Some([40_500_000, 44_800_000, 12_000_000])
        );
        assert_eq!(parse("cpu 1 2 3 4", 100), Some([400_000, 700_000, 300_000]));
        for bad in ["", "cpu0 1 2 3 4", "cpu 1 2 3", "cpu 1 x 3 4"] {
            assert_eq!(parse(bad, 100), None);
        }
        assert_eq!(parse("cpu 1 2 3 4", 0), None);
    }
}
