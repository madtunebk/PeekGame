//! Export declarations for pdh.dll (Performance Data Helper).
use crate::api::system::pdh;
use crate::registry::Export;

macro_rules! pdh {
    (@link $name:literal,$scope:literal,$function:path $(,)?) => {
        link_symbol!("pdh.dll", $name, $function);
    };
    ($name:literal,$scope:literal,$function:path $(,)?) => {
        Export {
            dll: "pdh.dll",
            name: $name,
            category: "system",
            status: "partial",
            scope: $scope,
            address: Some(|| $function as *const std::ffi::c_void),
        }
    };
}

exports! {
    pdh!(
        "PdhOpenQueryA",
        "opens a performance data query container (ANSI)",
        pdh::open_query_a
    ),
    pdh!(
        "PdhOpenQueryW",
        "opens a performance data query container (Unicode)",
        pdh::open_query_w
    ),
    pdh!(
        "PdhOpenQueryH",
        "opens a performance data query container by data source handle",
        pdh::open_query_h
    ),
    pdh!(
        "PdhAddEnglishCounterW",
        "adds an English-language performance counter path to a query",
        pdh::add_english_counter_w
    ),
    pdh!(
        "PdhAddCounterW",
        "adds a localized performance counter path to a query (Unicode)",
        pdh::add_counter_w
    ),
    pdh!(
        "PdhAddCounterA",
        "adds a localized performance counter path to a query (ANSI)",
        pdh::add_counter_a
    ),
    pdh!(
        "PdhCollectQueryData",
        "collects current metric samples for all counters in a query",
        pdh::collect_query_data
    ),
    pdh!(
        "PdhGetFormattedCounterValue",
        "formats and returns the computed counter value (double/long/large)",
        pdh::get_formatted_counter_value
    ),
    pdh!(
        "PdhRemoveCounter",
        "removes a performance counter from its query container",
        pdh::remove_counter
    ),
    pdh!(
        "PdhCloseQuery",
        "closes a performance query and frees its associated counters",
        pdh::close_query
    ),
    pdh!(
        "PdhEnumObjectsA",
        "enumerates performance objects available on the system (ANSI MULTI_SZ)",
        pdh::enum_objects_a
    ),
    pdh!(
        "PdhEnumObjectsW",
        "enumerates performance objects available on the system (Unicode MULTI_SZ)",
        pdh::enum_objects_w
    ),
}
