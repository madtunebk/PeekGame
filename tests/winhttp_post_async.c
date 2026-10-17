#include <windows.h>
#include <winhttp.h>
#ifndef PORT
#define PORT 18081
#endif
static HANDLE finished;
static volatile DWORD failure, sends, writes, headers;
static const char body[] = "peek-post-body";
static void CALLBACK callback(HINTERNET request, DWORD_PTR context, DWORD status, void *info, DWORD size) {
    (void)context;
    if (status == WINHTTP_CALLBACK_STATUS_REQUEST_ERROR) {
        failure = size == sizeof(WINHTTP_ASYNC_RESULT) ? ((WINHTTP_ASYNC_RESULT *)info)->dwError : 999;
        SetEvent(finished);
    } else if (status == WINHTTP_CALLBACK_STATUS_SENDREQUEST_COMPLETE) {
        sends++;
        if (!WinHttpWriteData(request, body, sizeof(body)-1, NULL)) {
            failure = GetLastError(); SetEvent(finished);
        }
    } else if (status == WINHTTP_CALLBACK_STATUS_WRITE_COMPLETE) {
        writes++;
        if (size != sizeof(DWORD) || *(DWORD *)info != sizeof(body)-1 ||
            !WinHttpReceiveResponse(request, NULL)) {
            failure = GetLastError() ? GetLastError() : 999; SetEvent(finished);
        }
    } else if (status == WINHTTP_CALLBACK_STATUS_HEADERS_AVAILABLE) {
        headers++;
        DWORD code = 0, length = sizeof(code);
        if (!WinHttpQueryHeaders(request, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            NULL, &code, &length, NULL) || code != 201) failure = 999;
        SetEvent(finished);
    }
}
void entry(void) {
    finished = CreateEventA(NULL, TRUE, FALSE, NULL);
    if (!finished) ExitProcess(1);
    HINTERNET session = WinHttpOpen(L"peek-post-async", WINHTTP_ACCESS_TYPE_NO_PROXY, NULL, NULL, WINHTTP_FLAG_ASYNC);
    if (!session) ExitProcess(2);
    if (WinHttpSetStatusCallback(session, callback, WINHTTP_CALLBACK_FLAG_ALL_COMPLETIONS, 0) == WINHTTP_INVALID_STATUS_CALLBACK) ExitProcess(3);
    HINTERNET connection = WinHttpConnect(session, L"127.0.0.1", PORT, 0);
    if (!connection) ExitProcess(4);
    HINTERNET request = WinHttpOpenRequest(connection, L"POST", L"/post", NULL, NULL, NULL, 0);
    if (!request) ExitProcess(5);
    if (!WinHttpAddRequestHeaders(request, L"Content-Type: application/json", (DWORD)-1, WINHTTP_ADDREQ_FLAG_ADD)) ExitProcess(6);
    if (!WinHttpSendRequest(request, NULL, 0, NULL, 0, sizeof(body)-1, 0)) ExitProcess(7);
    if (WaitForSingleObject(finished, 5000) != WAIT_OBJECT_0) ExitProcess(8);
    if (failure || sends != 1 || writes != 1 || headers != 1) ExitProcess(9);
    WinHttpCloseHandle(request);
    WinHttpCloseHandle(connection);
    WinHttpCloseHandle(session);
    ExitProcess(0);
}
