#ifndef PEEKGAME_IMPORTS_H
#define PEEKGAME_IMPORTS_H
#include <stdint.h>
#include <stddef.h>
/* Host loader ABI: ordinary Linux calling convention. */
uint32_t peekgame_abi_version(void);
uint32_t peekgame_start_import_trace(const char *path);
uint32_t peekgame_configure_process(uintptr_t image_base, const char *command_line);
/* Return is NULL for missing APIs. All non-NULL import pointers use ms_abi. */
const void *peekgame_resolve(const char *dll, const char *name);
struct peekgame_native_window { void *display; unsigned long window; };
/* Linux host access only. The Windows HWND remains a private token. Owner thread only. */
uint32_t peekgame_x11_window(uintptr_t hwnd, struct peekgame_native_window *output);
#endif
