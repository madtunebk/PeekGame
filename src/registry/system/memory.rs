//! Export declarations for memory. Implementation stays in api/.
use crate::api::memory::global;
use crate::api::memory::heap;
use crate::api::memory::mapping;
use crate::api::memory::virtual_memory;
use crate::registry::Export;

exports! {
    implemented!("HeapCreate", "memory", "partial", "private growable non-executable heaps; fixed-size and exception flags unsupported", heap::create),
    implemented!("HeapDestroy", "memory", "implemented", "releases private heap and all owned allocations", heap::destroy),
    implemented!("HeapSetInformation", "memory", "partial", "validates handle; unsupported policies return ERROR_NOT_SUPPORTED", heap::set_information),
    implemented!(
        "CreateFileMappingW",
        "memory",
        "partial",
        "files and pagefile-backed memory; names within this process",
        mapping::create_w
    ),
    implemented!(
        "CreateFileMappingA",
        "memory",
        "partial",
        "files and pagefile-backed memory; names within this process",
        mapping::create_a
    ),
    implemented!(
        "OpenFileMappingW",
        "memory",
        "partial",
        "names created by this process",
        mapping::open_w
    ),
    implemented!(
        "MapViewOfFile",
        "memory",
        "partial",
        "shared, copy-on-write and executable views",
        mapping::map_view
    ),
    implemented!(
        "MapViewOfFileEx",
        "memory",
        "partial",
        "as MapViewOfFile, at a requested free address",
        mapping::map_view_ex
    ),
    implemented!(
        "UnmapViewOfFile",
        "memory",
        "partial",
        "by view base",
        mapping::unmap_view
    ),
    implemented!(
        "FlushViewOfFile",
        "memory",
        "partial",
        "msync of the view's pages",
        mapping::flush_view
    ),
    implemented!(
        "GlobalAlloc",
        "memory",
        "partial",
        "fixed and moveable blocks",
        global::alloc
    ),
    implemented!(
        "GlobalLock",
        "memory",
        "implemented",
        "lock count",
        global::lock
    ),
    implemented!(
        "GlobalUnlock",
        "memory",
        "implemented",
        "lock count",
        global::unlock
    ),
    implemented!(
        "GlobalFree",
        "memory",
        "implemented",
        "unlocked blocks",
        global::free
    ),
    implemented!(
        "GlobalSize",
        "memory",
        "implemented",
        "block size",
        global::size
    ),
    implemented!(
        "VirtualAlloc",
        "memory",
        "partial",
        "reserve/commit as anonymous mappings on 64 KiB boundaries",
        virtual_memory::alloc
    ),
    implemented!(
        "VirtualFree",
        "memory",
        "partial",
        "release whole reservations, decommit pages",
        virtual_memory::free
    ),
    implemented!(
        "VirtualProtect",
        "memory",
        "implemented",
        "mprotect with the previous protection from /proc/self/maps",
        virtual_memory::protect
    ),
    implemented!(
        "VirtualQuery",
        "memory",
        "partial",
        "regions from /proc/self/maps; image and reservation bases",
        virtual_memory::query
    ),
    implemented!(
        "VirtualQueryEx",
        "memory",
        "partial",
        "current process memory query",
        virtual_memory::query_ex
    ),
    implemented!(
        "ReadProcessMemory",
        "memory",
        "partial",
        "current process readable memory ranges",
        virtual_memory::read_process_memory
    ),
    implemented!(
        "GetProcessHeap",
        "memory",
        "partial",
        "the single process heap",
        heap::get_process_heap
    ),
    implemented!(
        "HeapAlloc",
        "memory",
        "partial",
        "process heap; zero-fill; other heaps stop",
        heap::alloc
    ),
    implemented!(
        "HeapFree",
        "memory",
        "partial",
        "process heap blocks; foreign pointers fail",
        heap::release
    ),
    implemented!(
        "HeapSize",
        "memory",
        "partial",
        "requested size of process heap blocks",
        heap::size
    ),
    implemented!(
        "HeapReAlloc",
        "memory",
        "partial",
        "zero-fill and in-place-only; exceptions unavailable",
        heap::reallocate
    ),
    implemented!(
        "HeapQueryInformation",
        "memory",
        "partial",
        "HeapCompatibilityInformation (standard heap)",
        heap::query_information
    ),
}
