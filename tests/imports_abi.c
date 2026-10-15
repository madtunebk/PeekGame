/* Linux host loader -> real Windows x64 function pointers from the Rust .so. */
#include <assert.h>
#include <dlfcn.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>
#define WIN __attribute__((ms_abi))
typedef uint32_t (WIN *ErrorFn)(void);
typedef uintptr_t (WIN *StdFn)(uint32_t);
typedef int32_t (WIN *WriteFn)(uintptr_t, const void *, uint32_t, uint32_t *, void *);
static ErrorFn error;
static StdFn standard;
static void *worker(void *ignored) {
    (void)ignored;
    assert(error() == 0);
    assert(standard(7) == UINTPTR_MAX && error() == 87);
    return NULL;
}
int main(int argc, char **argv) {
    assert(argc == 2);
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) { fprintf(stderr, "%s\n", dlerror()); return 1; }
    const void *(*resolve)(const char *, const char *) = dlsym(library, "peekgame_resolve");
    uint32_t (*configure)(uintptr_t, const char *) = dlsym(library, "peekgame_configure_process");
    uint32_t (*version)(void) = dlsym(library, "peekgame_abi_version");
    assert(resolve && configure && version && version() == 1);
    error = (ErrorFn)resolve("KeRnEl32", "GetLastError");
    standard = (StdFn)resolve("kernel32.dll", "GetStdHandle");
    char *(WIN *command)(void) = (void *)resolve("kernel32.dll", "GetCommandLineA");
    uintptr_t (WIN *module)(const uint16_t *) = (void *)resolve("kernel32.dll", "GetModuleHandleW");
    WriteFn write_file = (WriteFn)resolve("kernel32.dll", "WriteFile");
    void (WIN *debug)(const char *) = (void *)resolve("kernel32.dll", "OutputDebugStringA");
    void (WIN *exit_process)(uint32_t) = (void *)resolve("kernel32.dll", "ExitProcess");
    assert(error && standard && command && module && write_file && debug && exit_process);
    assert(!resolve("d3d11.dll", "NotAnExport"));
    assert(!resolve("imaginary.dll", "GetLastError"));
    assert(!resolve("kernel32.dll", "getlasterror"));
    assert(!resolve(NULL, "GetLastError"));
    assert(command() == NULL && error() == 5023);
    assert(module(NULL) == 0 && error() == 5023);
    assert(configure(0, "game") == 87);
    static unsigned char mapped_image[4096];
    char line[] = "\"testgame.exe\" --frames 7";
    assert(configure((uintptr_t)mapped_image, line) == 0);
    line[0] = '!'; /* the library owns a copy, not a dangling caller pointer */
    assert(strcmp(command(), "\"testgame.exe\" --frames 7") == 0);
    assert(command() == command());
    assert(configure((uintptr_t)mapped_image, "other") == 183);
    assert(module(NULL) == (uintptr_t)mapped_image);
    uint16_t named[] = {'x', 0};
    assert(module(named) == 0 && error() == 126);
    uintptr_t out = standard((uint32_t)-11);
    assert(out != 0 && out != UINTPTR_MAX);
    assert(standard(7) == UINTPTR_MAX && error() == 87);
    int pipefd[2]; assert(pipe(pipefd) == 0);
    int saved = dup(1); assert(saved >= 0 && dup2(pipefd[1], 1) == 1);
    const char bytes[] = "ABI bytes from the game";
    uint32_t written = 99;
    assert(write_file(out, bytes, sizeof bytes, &written, NULL) == 1);
    assert(written == sizeof bytes);
    assert(dup2(saved, 1) == 1); close(saved); close(pipefd[1]);
    char received[sizeof bytes];
    assert(read(pipefd[0], received, sizeof received) == sizeof received);
    assert(memcmp(bytes, received, sizeof bytes) == 0); close(pipefd[0]);
    assert(!write_file(out, bytes, 1, &written, (void *)1) && written == 0 && error() == 50);
    assert(!write_file(out, NULL, 1, &written, NULL) && error() == 87);
    assert(write_file(out, NULL, 0, &written, NULL) && written == 0);
    assert(!write_file(out, bytes, 1, NULL, NULL) && error() == 87);
    assert(!write_file(7, bytes, 1, &written, NULL) && written == 0 && error() == 6);
    pthread_t thread; assert(pthread_create(&thread, NULL, worker, NULL) == 0);
    assert(pthread_join(thread, NULL) == 0 && error() == 6);
    assert(pipe(pipefd) == 0); saved = dup(2);
    assert(saved >= 0 && dup2(pipefd[1], 2) == 2);
    debug("debug text");
    assert(error() == 6);
    assert(dup2(saved, 2) == 2); close(saved); close(pipefd[1]);
    char message[10]; assert(read(pipefd[0], message, sizeof message) == sizeof message);
    assert(memcmp(message, "debug text", sizeof message) == 0); close(pipefd[0]);
    pid_t child = fork(); assert(child >= 0);
    if (child == 0) { exit_process(0x123); _exit(99); }
    int status; assert(waitpid(child, &status, 0) == child);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 0x23);
    dlclose(library);
    puts("PASS: shared-library Windows ABI, arguments, byte writes, errors, TLS, and exit");
    return 0;
}
