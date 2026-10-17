//! Export declarations for exceptions. Implementation stays in api/.
use crate::registry::Export;
use crate::seh;

exports! {
    implemented!("RtlCaptureStackBackTrace", "exception", "partial", "PE/JIT unwind metadata, skip/count/hash; stops at host boundary, max 4096 frames", crate::seh::backtrace::capture_entry),
    export!("ntdll.dll", "RtlCaptureStackBackTrace", "exception", "partial", "same bounded Windows stack capture as kernel32", crate::seh::backtrace::capture_entry),
    implemented!(
        "RaiseException",
        "system",
        "partial",
        "vectored handlers, frame handlers through the unwind tables, then the unhandled-exception filter",
        seh::raise_exception_entry
    ),
    implemented!(
        "DebugBreak",
        "system",
        "implemented",
        "raises STATUS_BREAKPOINT (0x80000003)",
        crate::api::system::debug::debug_break
    ),
    implemented!(
        "RtlUnwindEx",
        "system",
        "partial",
        "termination handlers up to the target frame, then continues there",
        seh::unwind_ex_entry
    ),
    implemented!(
        "RtlUnwind",
        "system",
        "partial",
        "as RtlUnwindEx",
        seh::unwind_entry
    ),
    implemented!(
        "RtlCaptureContext",
        "system",
        "partial",
        "integer, flags, segment and FPU/SSE state",
        seh::capture_context_entry
    ),
    implemented!(
        "RtlRestoreContext",
        "system",
        "partial",
        "integer, flags and FPU/SSE state",
        seh::restore_context_entry
    ),
    implemented!(
        "RtlVirtualUnwind",
        "system",
        "partial",
        "UNWIND_INFO v1/v2, chained info and epilogs",
        seh::unwind::virtual_unwind
    ),
    implemented!(
        "RtlLookupFunctionEntry",
        "system",
        "partial",
        "registered tables, then the image's .pdata",
        seh::tables::lookup
    ),
    implemented!(
        "RtlAddFunctionTable",
        "system",
        "partial",
        "caller-owned sorted array",
        seh::tables::add
    ),
    implemented!(
        "RtlDeleteFunctionTable",
        "system",
        "partial",
        "arrays and callback tables",
        seh::tables::delete
    ),
    implemented!(
        "RtlInstallFunctionTableCallback",
        "system",
        "partial",
        "callback asked per pc in its range",
        seh::tables::install_callback
    ),
    implemented!(
        "RtlAddGrowableFunctionTable",
        "system",
        "partial",
        "sorted table grown in place",
        seh::tables::add_growable
    ),
    implemented!(
        "RtlGrowFunctionTable",
        "system",
        "partial",
        "extends the entry count",
        seh::tables::grow
    ),
    implemented!(
        "RtlDeleteGrowableFunctionTable",
        "system",
        "partial",
        "removes the table",
        seh::tables::delete_growable
    ),
    implemented!(
        "RtlPcToFileHeader",
        "system",
        "partial",
        "loaded PE images",
        seh::tables::pc_to_file_header
    ),
    implemented!(
        "AddVectoredExceptionHandler",
        "system",
        "partial",
        "first and last lists",
        seh::add_vectored_handler
    ),
    implemented!(
        "RemoveVectoredExceptionHandler",
        "system",
        "partial",
        "by registration handle",
        seh::remove_vectored_handler
    ),
    implemented!(
        "__C_specific_handler",
        "system",
        "partial",
        "__try/__except/__finally scope tables",
        seh::c_specific_handler
    ),
    export!(
        "ntdll.dll",
        "RtlUnwindEx",
        "system",
        "partial",
        "termination handlers up to the target frame, then continues there",
        seh::unwind_ex_entry
    ),
    export!(
        "ntdll.dll",
        "RtlUnwind",
        "system",
        "partial",
        "as RtlUnwindEx",
        seh::unwind_entry
    ),
    export!(
        "ntdll.dll",
        "RtlCaptureContext",
        "system",
        "partial",
        "integer, flags, segment and FPU/SSE state",
        seh::capture_context_entry
    ),
    export!(
        "ntdll.dll",
        "RtlRestoreContext",
        "system",
        "partial",
        "integer, flags and FPU/SSE state",
        seh::restore_context_entry
    ),
    export!(
        "ntdll.dll",
        "RtlVirtualUnwind",
        "system",
        "partial",
        "UNWIND_INFO v1/v2, chained info and epilogs",
        seh::unwind::virtual_unwind
    ),
    export!(
        "ntdll.dll",
        "RtlLookupFunctionEntry",
        "system",
        "partial",
        "registered tables, then the image's .pdata",
        seh::tables::lookup
    ),
    export!(
        "ntdll.dll",
        "RtlAddFunctionTable",
        "system",
        "partial",
        "caller-owned sorted array",
        seh::tables::add
    ),
    export!(
        "ntdll.dll",
        "RtlDeleteFunctionTable",
        "system",
        "partial",
        "arrays and callback tables",
        seh::tables::delete
    ),
    export!(
        "ntdll.dll",
        "RtlInstallFunctionTableCallback",
        "system",
        "partial",
        "callback asked per pc in its range",
        seh::tables::install_callback
    ),
    export!(
        "ntdll.dll",
        "RtlAddGrowableFunctionTable",
        "system",
        "partial",
        "sorted table grown in place",
        seh::tables::add_growable
    ),
    export!(
        "ntdll.dll",
        "RtlGrowFunctionTable",
        "system",
        "partial",
        "extends the entry count",
        seh::tables::grow
    ),
    export!(
        "ntdll.dll",
        "RtlDeleteGrowableFunctionTable",
        "system",
        "partial",
        "removes the table",
        seh::tables::delete_growable
    ),
    export!(
        "ntdll.dll",
        "RtlPcToFileHeader",
        "system",
        "partial",
        "loaded PE images",
        seh::tables::pc_to_file_header
    ),
    export!(
        "ntdll.dll",
        "RtlAddVectoredExceptionHandler",
        "system",
        "partial",
        "first and last lists",
        seh::add_vectored_handler
    ),
    export!(
        "ntdll.dll",
        "RtlRemoveVectoredExceptionHandler",
        "system",
        "partial",
        "by registration handle",
        seh::remove_vectored_handler
    ),
    export!(
        "ntdll.dll",
        "__C_specific_handler",
        "system",
        "partial",
        "__try/__except/__finally scope tables",
        seh::c_specific_handler
    ),
    implemented!(
        "UnhandledExceptionFilter",
        "system",
        "partial",
        "the top-level filter, else execute handler",
        seh::unhandled_exception_filter
    ),
    export!(
        "msvcrt.dll",
        "__C_specific_handler",
        "crt",
        "partial",
        "__try/__except/__finally scope tables",
        seh::c_specific_handler
    ),
}
