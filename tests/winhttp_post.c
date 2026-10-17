#include <windows.h>
#include <winhttp.h>
#ifndef PORT
#define PORT 18081
#endif
void entry(void) {
    const char body[] = "peek-post-body";
    HINTERNET session = WinHttpOpen(L"peek-post", WINHTTP_ACCESS_TYPE_NO_PROXY, NULL, NULL, 0);
    if (!session) ExitProcess(1);
    HINTERNET connection = WinHttpConnect(session, L"127.0.0.1", PORT, 0);
    if (!connection) ExitProcess(2);
    HINTERNET request = WinHttpOpenRequest(connection, L"POST", L"/post", NULL, NULL, NULL, 0);
    if (!request) ExitProcess(3);
    if (!WinHttpAddRequestHeaders(request, L"Content-Type: application/json", (DWORD)-1, WINHTTP_ADDREQ_FLAG_ADD)) ExitProcess(4);
#ifdef STREAM
    if (!WinHttpSendRequest(request, NULL, 0, NULL, 0, sizeof(body)-1, 0)) ExitProcess(5);
    DWORD written = 0;
    if (!WinHttpWriteData(request, body, 5, &written) || written != 5) ExitProcess(11);
    if (!WinHttpWriteData(request, body + 5, sizeof(body)-1-5, &written) || written != sizeof(body)-1-5) ExitProcess(12);
#else
    if (!WinHttpSendRequest(request, NULL, 0, (void *)body, sizeof(body)-1, sizeof(body)-1, 0)) ExitProcess(5);
#endif
    if (!WinHttpReceiveResponse(request, NULL)) ExitProcess(6);
    DWORD status = 0, size = sizeof(status);
    if (!WinHttpQueryHeaders(request, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, NULL, &status, &size, NULL)) ExitProcess(7);
    if (status != 201) ExitProcess(8);
    DWORD available = 0;
    if (!WinHttpQueryDataAvailable(request, &available) || available != 2) ExitProcess(9);
    char answer[8] = {0}; DWORD read = 0;
    if (!WinHttpReadData(request, answer, sizeof(answer), &read) || read != 2 || answer[0] != 'O' || answer[1] != 'K') ExitProcess(10);
    WinHttpCloseHandle(request);
    WinHttpCloseHandle(connection);
    WinHttpCloseHandle(session);
    ExitProcess(0);
}
