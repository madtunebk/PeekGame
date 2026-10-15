#pragma once
#include <cstdint>
/* Shared with Rust Surface: opaque to Windows code. Atomics accessed with GCC intrinsics. */
struct PeekSurface {void* display; uint64_t window; uint32_t alive,width,height,visible;};
static_assert(sizeof(PeekSurface)==32);
