# WinHTTP transport progress

`tools/WinHTTP/winhttp_probe.rs` is a minimal Windows x64 PE written in Rust.
It uses WinHTTP directly without Rust's standard library or a Windows CRT.
Build and run it with `python3 tools/WinHTTP/build_winhttp_probe.py --run`.
The PE is saved under `cache/winhttp-probe/`. Exit 0 means HTTPS GET
returned HTTP 200 with a nonempty body; other nonzero codes identify the
failing step in `probe()`.

The current PeekGame build provides synchronous and asynchronous GET/HEAD/POST
through the host's `libcurl.so.4`, with TLS certificate and hostname checks.
Request headers added with supported ADD/REPLACE modifiers are sent. The trace
`PEEKGAME_TRACE_HTTP=1` logs request methods, URLs, header names with values
redacted, HTTP status codes and transport errors.

`python3 tests/test_winhttp_transport.py` passed against a private local
server: the Rust PE read the full GET body, and a Windows async HEAD fixture
received SENDREQUEST_COMPLETE, HEADERS_AVAILABLE, numeric status 200 and
DATA_AVAILABLE and READ_COMPLETE for a zero-byte async read, plus
Content-Type from the server response. The fixture
verified that `Connection: close` reached the server. `python3 tests/test_winhttp_offline.py` passed for refused connections
and callback errors. The public HTTPS probe also exited 0.

DOOM's real async HEAD request reached the service and returned HTTP 200.
Its following WinHttpReadData call exposed a missing async READ_COMPLETE path,
which the local fixture now covers. The owner still needs to rerun DOOM with
this build. HTTP 200 alone does not establish online login or gameplay.

DOOM then exposed response-header queries: `CONTENT_TYPE`, `CONTENT_LENGTH`,
status text, encoding, raw headers and date fields. PeekGame now captures real
HTTP response headers and answers supported string/numeric WinHTTP queries.
SYSTEMTIME date conversion remains unsupported. The owner has not yet rerun
DOOM with this response-header build.

DOOM reached Bethesda and PlayFab authentication endpoints. The next log showed
their POST requests use `WinHttpSendRequest` with zero inline bytes and a declared
body length, followed by `WinHttpWriteData`. PeekGame now buffers bounded request
bodies across writes and sends them when `WinHttpReceiveResponse` is called.
Asynchronous handles deliver SENDREQUEST_COMPLETE, WRITE_COMPLETE and then
HEADERS_AVAILABLE in order. Request bodies and header values are never traced.
`python3 tests/test_winhttp_post.py` passed against a private local server for
inline, fragmented synchronous and asynchronous POST, including request body,
headers, HTTP 201 and callbacks. `python3 tests/test_winhttp_transport.py` also
passed. The owner has not yet checked Bethesda or PlayFab login with this build.
