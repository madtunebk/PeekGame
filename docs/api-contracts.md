# API contract references

Local Microsoft documentation provided by the owner:

- `/home/nobus/projects/sdl3-window/win32/desktop-src/Debug/last-error-code.md`:
  errors are per thread; APIs only change that state under their documented rules.
- `/home/nobus/projects/sdl3-window/win32/desktop-src/ProcThread/terminating-a-process.md`:
  process exit ends all threads and notifies attached PE DLLs. The first Rust backend
  performs host process exit; PE DLL detach notifications still need the PE loader.
- `/home/nobus/projects/sdl3-window/win32/desktop-src/FileIO/synchronous-and-asynchronous-i-o.md`:
  synchronous and overlapped operations have distinct contracts. Overlapped I/O is
  explicitly unsupported in the first Rust WriteFile implementation.

Function signatures and parameter details:

- [GetStdHandle](https://learn.microsoft.com/en-us/windows/console/getstdhandle)
- [WriteFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile)
- [GetModuleHandleW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlew)

For each new API, add its implementation under a subsystem and record its supported
scope in `src/registry.rs`. A catalog entry without a callable pointer must never
claim an implementation. Unknown APIs are not auto-aliased or replaced by success stubs.

Window ABI and behavior references:

- Local `/usr/include/X11/Xlib.h` and `/usr/include/X11/Xutil.h`: native event
  structures and function signatures (not Windows-facing structures).
- Local `win32/desktop-src/winmsg/using-window-classes.md`,
  `wm-nccreate.md`, `wm-create.md`, `wm-ncdestroy.md`, and
  `about-messages-and-message-queues.md` under sdl3-window: class lifecycle,
  creation/destruction notifications, and message processing.
- `tests/testgame/README.md`: the explicit client-dimension/compositor-decoration
  model for this prototype. This scope is reported as partial USER32 support.
