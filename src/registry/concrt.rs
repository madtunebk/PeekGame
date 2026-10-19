//! A narrow CONCRT140 surface for Palworld's concurrent queue base.
use super::Export;
exports! {
    export_registry_only!("concrt140.dll", "??0_Concurrent_queue_base_v4@details@Concurrency@@IEAA@_K@Z", "crt", "partial", "constructs the queue base and zeroes its queue state; derived constructor installs item-specific vtable", crate::api::concurrency::concurrent_queue_base_v4_ctor),
    export_registry_only!("concrt140.dll", "??1_Concurrent_queue_base_v4@details@Concurrency@@MEAA@XZ", "crt", "partial", "releases the queue base state", crate::api::concurrency::concurrent_queue_base_v4_dtor),
    export_registry_only!("concrt140.dll", "?_Internal_push@_Concurrent_queue_base_v4@details@Concurrency@@IEAAXPEBX@Z", "crt", "partial", "copies one item into the queue's first synchronized lane", crate::api::concurrency::concurrent_queue_base_v4_internal_push),
    export_registry_only!("concrt140.dll", "?_Internal_pop_if_present@_Concurrent_queue_base_v4@details@Concurrency@@IEAA_NPEAX@Z", "crt", "partial", "pops an item from the first synchronized lane when available", crate::api::concurrency::concurrent_queue_base_v4_internal_pop_if_present),
}
