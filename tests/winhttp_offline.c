/* WinHTTP without a transport: handles, options, sync failure codes and async
   REQUEST_ERROR/HANDLE_CLOSING callbacks on another thread. Exit 0 or the step. */
#include <windows.h>
#include <winhttp.h>

static HANDLE done_event, closed_event;
static DWORD main_thread, callback_thread, seen_status, async_error;
static DWORD_PTR async_api, seen_context, created_count;
static HINTERNET closed_handle;

static void CALLBACK status(HINTERNET handle, DWORD_PTR context, DWORD code, LPVOID info, DWORD length) {
    if (code == WINHTTP_CALLBACK_STATUS_HANDLE_CREATED) {
        created_count++;
        return;
    }
    if (code == WINHTTP_CALLBACK_STATUS_REQUEST_ERROR && length == sizeof(WINHTTP_ASYNC_RESULT)) {
        WINHTTP_ASYNC_RESULT *result = info;
        callback_thread = GetCurrentThreadId();
        seen_status = code;
        seen_context = context;
        async_api = result->dwResult;
        async_error = result->dwError;
        SetEvent(done_event);
    }
    if (code == WINHTTP_CALLBACK_STATUS_HANDLE_CLOSING && length == sizeof(HINTERNET)) {
        closed_handle = *(HINTERNET *)info;
        SetEvent(closed_event);
    }
    (void)handle;
}

static int check(void) {
    DWORD value = 7;
    main_thread = GetCurrentThreadId();
    /* Synchronous session. */
    HINTERNET session = WinHttpOpen(L"peekgame-test", WINHTTP_ACCESS_TYPE_NO_PROXY, NULL, NULL, 0);
    if (!session) return 1;
    if (!WinHttpSetTimeouts(session, 1000, 1000, 1000, 1000)) return 2;
    if (WinHttpSetTimeouts(session, -2, 0, 0, 0) || GetLastError() != ERROR_INVALID_PARAMETER) return 3;
    if (!WinHttpSetOption(session, WINHTTP_OPTION_CONNECT_TIMEOUT, &value, 4)) return 4;
    if (WinHttpSetOption(session, 0x7777, &value, 4) || GetLastError() != ERROR_WINHTTP_INVALID_OPTION) return 5;
    HINTERNET connection = WinHttpConnect(session, L"example.invalid", 443, 0);
    if (!connection) return 6;
    if (WinHttpOpenRequest(session, L"GET", L"/", NULL, NULL, NULL, 0) || GetLastError() != ERROR_WINHTTP_INCORRECT_HANDLE_TYPE) return 7;
    HINTERNET request = WinHttpOpenRequest(connection, L"GET", L"/status", NULL, WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES, WINHTTP_FLAG_SECURE);
    if (!request) return 8;
    if (!WinHttpAddRequestHeaders(request, L"X-Test: 1", (DWORD)-1, WINHTTP_ADDREQ_FLAG_ADD)) return 9;
    if (WinHttpSendRequest(request, NULL, 0, NULL, 0, 0, 0) || GetLastError() != ERROR_WINHTTP_CANNOT_CONNECT) return 10;
    if (WinHttpReceiveResponse(request, NULL) || GetLastError() != ERROR_WINHTTP_INCORRECT_HANDLE_STATE) return 11;
    if (WinHttpReadData(request, &value, 4, &value) || GetLastError() != ERROR_WINHTTP_INCORRECT_HANDLE_STATE) return 12;
    if (!WinHttpCloseHandle(request) || !WinHttpCloseHandle(connection) || !WinHttpCloseHandle(session)) return 13;
    if (WinHttpCloseHandle(session) || GetLastError() != ERROR_INVALID_HANDLE) return 14;
    /* Asynchronous session: errors and closing arrive on the callback thread. */
    done_event = CreateEventA(NULL, TRUE, FALSE, NULL);
    closed_event = CreateEventA(NULL, TRUE, FALSE, NULL);
    session = WinHttpOpen(L"peekgame-test", WINHTTP_ACCESS_TYPE_NO_PROXY, NULL, NULL, WINHTTP_FLAG_ASYNC);
    if (!session) return 15;
    if (WinHttpSetStatusCallback(session, status, WINHTTP_CALLBACK_FLAG_ALL_NOTIFICATIONS, 0) != NULL) return 16;
    connection = WinHttpConnect(session, L"example.invalid", 443, 0);
    request = WinHttpOpenRequest(connection, L"POST", L"/login", NULL, NULL, NULL, WINHTTP_FLAG_SECURE);
    if (!connection || !request || created_count != 2) return 17;
    if (!WinHttpSendRequest(request, NULL, 0, NULL, 0, 0, 0x1234)) return 18;
    if (WaitForSingleObject(done_event, 5000) != WAIT_OBJECT_0) return 19;
    if (seen_status != WINHTTP_CALLBACK_STATUS_REQUEST_ERROR || seen_context != 0x1234) return 20;
    if (async_api != API_SEND_REQUEST || async_error != ERROR_WINHTTP_CANNOT_CONNECT) return 21;
    if (callback_thread == main_thread || callback_thread == 0) return 22;
    if (!WinHttpCloseHandle(request)) return 23;
    if (WaitForSingleObject(closed_event, 5000) != WAIT_OBJECT_0 || closed_handle != request) return 24;
    WinHttpCloseHandle(connection);
    WinHttpCloseHandle(session);
    return 0;
}

void entry(void) { ExitProcess(check()); }
