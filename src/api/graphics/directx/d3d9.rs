//! Direct3D 9 performance and profiling markers (D3DPERF).

/// Begins a user-defined event. Returns the event level (0-based).
pub extern "win64" fn begin_event(_col: u32, _name: *const u16) -> i32 {
    0
}

/// Ends a user-defined event. Returns the event level that was ended.
pub extern "win64" fn end_event() -> i32 {
    0
}

/// Sets a user-defined marker.
pub extern "win64" fn set_marker(_col: u32, _name: *const u16) {}

/// Sets profiling options.
pub extern "win64" fn set_options(_options: u32) {}

/// Gets status (0 = profiler not active).
pub extern "win64" fn get_status() -> u32 {
    0
}

/// Queries repeat frame.
pub extern "win64" fn query_repeat_frame() -> i32 {
    0
}

/// Direct3DCreate9 stub
pub extern "win64" fn direct3d_create9(_sdk_version: u32) -> usize {
    0
}
