//! Export declarations for time. Implementation stays in api/.
use crate::api::system::time;
use crate::registry::Export;

exports! {
    implemented!("GetSystemTimes", "system", "partial", "host aggregate CPU times in 100 ns units", crate::api::system::cpu_times::get),
    implemented!(
        "GetTimeZoneInformation",
        "system",
        "partial",
        "host zone database; transitions found by scanning this year",
        time::time_zone_information
    ),
    implemented!(
        "GetDynamicTimeZoneInformation",
        "system",
        "partial",
        "as GetTimeZoneInformation; key name is the standard name",
        time::dynamic_time_zone_information
    ),
    implemented!(
        "SystemTimeToTzSpecificLocalTime",
        "system",
        "partial",
        "current time zone only",
        time::system_time_to_tz_local
    ),
    implemented!(
        "TzSpecificLocalTimeToSystemTime",
        "system",
        "partial",
        "current time zone only",
        time::tz_local_to_system_time
    ),
    implemented!(
        "CompareFileTime",
        "system",
        "partial",
        "complete",
        time::compare_file_time
    ),
    implemented!(
        "FileTimeToLocalFileTime",
        "system",
        "partial",
        "offset in effect at that time",
        time::file_time_to_local
    ),
    implemented!(
        "LocalFileTimeToFileTime",
        "system",
        "partial",
        "offset in effect at that time",
        time::local_to_file_time
    ),
    implemented!(
        "GetTickCount",
        "system",
        "implemented",
        "milliseconds since boot (CLOCK_BOOTTIME)",
        time::tick_count
    ),
    implemented!(
        "GetTickCount64",
        "system",
        "implemented",
        "milliseconds since boot (CLOCK_BOOTTIME)",
        time::tick_count64
    ),
    implemented!(
        "QueryUnbiasedInterruptTime",
        "system",
        "implemented",
        "100 ns monotonic time (CLOCK_MONOTONIC)",
        time::query_unbiased_interrupt_time
    ),
    implemented!(
        "GetSystemTimePreciseAsFileTime",
        "system",
        "implemented",
        "host UTC with 100 ns resolution",
        time::precise_file_time
    ),
    implemented!(
        "GetSystemTime",
        "system",
        "implemented",
        "host UTC as SYSTEMTIME",
        time::system_time
    ),
    implemented!(
        "GetLocalTime",
        "system",
        "implemented",
        "host local time as SYSTEMTIME",
        time::local_time
    ),
    implemented!(
        "FileTimeToSystemTime",
        "system",
        "implemented",
        "UTC conversion with Windows validation",
        time::file_time_to_system_time
    ),
    implemented!(
        "SystemTimeToFileTime",
        "system",
        "implemented",
        "UTC conversion with Windows validation",
        time::system_time_to_file_time
    ),
}
