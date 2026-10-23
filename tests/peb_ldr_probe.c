#include <stdint.h>

__declspec(dllimport) void __stdcall ExitProcess(unsigned int);

void mainCRTStartup(void) {
    uintptr_t peb;
    __asm__ volatile("movq %%gs:0x60, %0" : "=r"(peb));
    if (!peb) ExitProcess(1);
    uintptr_t image = *(uintptr_t *)(peb + 0x10);
    uintptr_t ldr = *(uintptr_t *)(peb + 0x18);
    if (!image || !ldr) ExitProcess(2);
    uintptr_t head = ldr + 0x10;
    uintptr_t first = *(uintptr_t *)head;
    if (first == head || !first) ExitProcess(3);
    if (*(uintptr_t *)(first + 0x30) != image) ExitProcess(4);
    if (*(uintptr_t *)(first + 8) != head) ExitProcess(5);
    if (*(uintptr_t *)(*(uintptr_t *)(head + 8)) != head) ExitProcess(6);
    ExitProcess(0);
}
