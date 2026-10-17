/* ReadFileScatter into shuffled pages at an OVERLAPPED offset, with event,
   GetOverlappedResult and an IOCP packet; exit code 0 on success, else the step. */
#include <windows.h>

static int check(void) {
    char path[] = "Z:\\tmp\\peekgame-read-scatter.bin";
    static unsigned char data[5 * 4096];
    DWORD done, key;
    ULONG_PTR completion_key;
    OVERLAPPED *packet;
    for (int i = 0; i < (int)sizeof data; i++) data[i] = (unsigned char)(i * 13 + i / 4096);
    HANDLE f = CreateFileA(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    if (f == INVALID_HANDLE_VALUE || !WriteFile(f, data, sizeof data, &done, NULL)) return 1;
    CloseHandle(f);
    f = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING,
                    FILE_FLAG_OVERLAPPED | FILE_FLAG_NO_BUFFERING, NULL);
    if (f == INVALID_HANDLE_VALUE) return 2;
    HANDLE port = CreateIoCompletionPort(f, NULL, 77, 0);
    if (!port) return 3;
    unsigned char *pages = VirtualAlloc(NULL, 3 * 4096, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    FILE_SEGMENT_ELEMENT segments[4] = {0};
    segments[0].Buffer = PtrToPtr64(pages + 2 * 4096);
    segments[1].Buffer = PtrToPtr64(pages);
    segments[2].Buffer = PtrToPtr64(pages + 4096);
    OVERLAPPED ov = {0};
    ov.Offset = 4096;
    ov.hEvent = CreateEventA(NULL, TRUE, FALSE, NULL);
    if (!ReadFileScatter(f, segments, 3 * 4096, NULL, &ov) && GetLastError() != ERROR_IO_PENDING) return 4;
    if (WaitForSingleObject(ov.hEvent, 5000) != WAIT_OBJECT_0) return 5;
    if (!GetOverlappedResult(f, &ov, &done, TRUE) || done != 3 * 4096) return 6;
    for (int i = 0; i < 4096; i++) {
        if (pages[2 * 4096 + i] != data[4096 + i]) return 7;
        if (pages[i] != data[2 * 4096 + i]) return 8;
        if (pages[4096 + i] != data[3 * 4096 + i]) return 9;
    }
    if (!GetQueuedCompletionStatus(port, &key, &completion_key, &packet, 1000)) return 10;
    if (completion_key != 77 || packet != &ov || key != 3 * 4096) return 11;
    /* A length that is not whole pages, and a missing OVERLAPPED, are refused. */
    if (ReadFileScatter(f, segments, 100, NULL, &ov) || GetLastError() != ERROR_INVALID_PARAMETER) return 12;
    if (ReadFileScatter(f, segments, 4096, NULL, NULL) || GetLastError() != ERROR_INVALID_PARAMETER) return 13;
    /* Reading across the end returns the bytes that exist. */
    ov.Offset = 4 * 4096;
    ResetEvent(ov.hEvent);
    if (!ReadFileScatter(f, segments, 2 * 4096, NULL, &ov) && GetLastError() != ERROR_IO_PENDING) return 14;
    if (!GetOverlappedResult(f, &ov, &done, TRUE) || done != 4096) return 15;
    for (int i = 0; i < 4096; i++)
        if (pages[2 * 4096 + i] != data[4 * 4096 + i]) return 16;
    CloseHandle(f);
    CloseHandle(port);
    DeleteFileW(L"Z:\\tmp\\peekgame-read-scatter.bin");
    return 0;
}

void entry(void) { ExitProcess(check()); }
