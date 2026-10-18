/* FindFirstChangeNotification backed by inotify: a change in the watched directory
   signals the handle; re-arm with FindNextChangeNotification; close releases it. */
#include <windows.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
static void touch(const wchar_t *name) {
    HANDLE f = CreateFileW(name, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, 0, NULL);
    if (f == INVALID_HANDLE_VALUE) ExitProcess(200);
    DWORD wrote; WriteFile(f, "hi", 2, &wrote, NULL); CloseHandle(f);
}
void entry(void) {
    CHECK(CreateDirectoryW(L"watchdir", NULL));
    HANDLE h = FindFirstChangeNotificationW(L"watchdir", FALSE,
        FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE);
    CHECK(h != INVALID_HANDLE_VALUE && h != NULL);
    /* nothing changed yet */
    CHECK(WaitForSingleObject(h, 300) == WAIT_TIMEOUT);
    /* a new file in the directory must signal the watch */
    touch(L"watchdir\\a.txt");
    CHECK(WaitForSingleObject(h, 5000) == WAIT_OBJECT_0);
    /* re-arm: the signal clears until the next change */
    CHECK(FindNextChangeNotification(h));
    CHECK(WaitForSingleObject(h, 300) == WAIT_TIMEOUT);
    touch(L"watchdir\\b.txt");
    CHECK(WaitForSingleObject(h, 5000) == WAIT_OBJECT_0);
    CHECK(FindCloseChangeNotification(h));
    /* a bad handle is rejected */
    CHECK(!FindNextChangeNotification(h) && GetLastError() == ERROR_INVALID_HANDLE);
    ExitProcess(0);
}
