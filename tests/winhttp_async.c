#include <windows.h>
#include <winhttp.h>
#ifndef PORT
#define PORT 18081
#endif
static HANDLE finished;
static volatile DWORD status_code, available, failure, send_seen, headers_seen, read_seen, read_length;
static char read_buffer[32];
static wchar_t content_type[64];
static void CALLBACK callback(HINTERNET request, DWORD_PTR context, DWORD status, void *info, DWORD size) {
    (void)context;
    if (status == WINHTTP_CALLBACK_STATUS_REQUEST_ERROR) {
        failure = size == sizeof(WINHTTP_ASYNC_RESULT) ? ((WINHTTP_ASYNC_RESULT *)info)->dwError : 999;
        SetEvent(finished);
    } else if (status == WINHTTP_CALLBACK_STATUS_SENDREQUEST_COMPLETE) {
        send_seen++;
        if (!WinHttpReceiveResponse(request, NULL)) { failure = GetLastError(); SetEvent(finished); }
    } else if (status == WINHTTP_CALLBACK_STATUS_HEADERS_AVAILABLE) {
        headers_seen++;
        DWORD length = sizeof(status_code);
        if (!WinHttpQueryHeaders(request, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                                 NULL, (void *)&status_code, &length, NULL)) {
            failure = GetLastError(); SetEvent(finished); return;
        }
        DWORD type_length = sizeof(content_type);
        if (!WinHttpQueryHeaders(request, WINHTTP_QUERY_CONTENT_TYPE, NULL,
                                 content_type, &type_length, NULL) ||
            lstrcmpW(content_type, L"application/json") != 0) {
            failure = GetLastError() ? GetLastError() : 999;
            SetEvent(finished); return;
        }
        if (!WinHttpQueryDataAvailable(request, NULL)) { failure = GetLastError(); SetEvent(finished); }
    } else if (status == WINHTTP_CALLBACK_STATUS_DATA_AVAILABLE) {
        if (size == sizeof(DWORD)) available = *(DWORD *)info;
        if (!WinHttpReadData(request, read_buffer, sizeof(read_buffer), NULL)) {
            failure = GetLastError(); SetEvent(finished);
        }
    } else if (status == WINHTTP_CALLBACK_STATUS_READ_COMPLETE) {
        read_seen++;
        read_length = size;
        if (info != read_buffer) failure = 999;
        SetEvent(finished);
    }
}
void entry(void) {
    finished = CreateEventA(NULL, TRUE, FALSE, NULL);
    if (!finished) ExitProcess(1);
    HINTERNET session = WinHttpOpen(L"peek-async", WINHTTP_ACCESS_TYPE_NO_PROXY, NULL, NULL, WINHTTP_FLAG_ASYNC);
    if (!session) ExitProcess(2);
    if (WinHttpSetStatusCallback(session, callback, WINHTTP_CALLBACK_FLAG_ALL_COMPLETIONS, 0) == WINHTTP_INVALID_STATUS_CALLBACK) ExitProcess(3);
    HINTERNET connection = WinHttpConnect(session, L"127.0.0.1", PORT, 0);
    if (!connection) ExitProcess(4);
    HINTERNET request = WinHttpOpenRequest(connection, L"HEAD", L"/", NULL, NULL, NULL, 0);
    if (!request) ExitProcess(5);
    if (!WinHttpAddRequestHeaders(request, L"Connection: close", (DWORD)-1, WINHTTP_ADDREQ_FLAG_REPLACE)) ExitProcess(6);
    if (!WinHttpSendRequest(request, NULL, 0, NULL, 0, 0, 0)) ExitProcess(7);
    if (WaitForSingleObject(finished, 5000) != WAIT_OBJECT_0) ExitProcess(8);
    if (failure) ExitProcess(9);
    if (!send_seen || !headers_seen || status_code != 200 || available != 0 || !read_seen || read_length != 0) ExitProcess(10);
    WinHttpCloseHandle(request);
    WinHttpCloseHandle(connection);
    WinHttpCloseHandle(session);
    ExitProcess(0);
}
