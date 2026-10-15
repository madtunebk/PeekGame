use super::Export;
use crate::api::crt::locale;
use crate::api::crt::stdio;
use crate::api::crt::{simple, startup, ucrt};
macro_rules! crt {
    (@link $name:literal,$scope:literal,$function:path $(,)?) => {
        link_symbol!("msvcrt.dll", $name, $function);
    };
    ($name:literal,$scope:literal,$function:path $(,)?) => {
        Export {
            dll: "msvcrt.dll",
            name: $name,
            category: "crt",
            status: "partial",
            scope: $scope,
            address: Some(|| $function as *const std::ffi::c_void),
        }
    };
}
exports! {
    crt!(
        "__stdio_common_vfprintf",
        "UCRT for MinGW-built Windows libraries",
        ucrt::common_vfprintf
    ),
    crt!(
        "__stdio_common_vfwprintf",
        "UCRT for MinGW-built Windows libraries",
        ucrt::common_vfwprintf
    ),
    crt!(
        "__stdio_common_vsprintf",
        "UCRT for MinGW-built Windows libraries",
        ucrt::common_vsprintf
    ),
    crt!(
        "__stdio_common_vswprintf",
        "UCRT for MinGW-built Windows libraries",
        ucrt::common_vswprintf
    ),
    crt!(
        "__acrt_iob_func",
        "UCRT for MinGW-built Windows libraries",
        ucrt::acrt_iob_func
    ),
    crt!(
        "fputc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::fputc
    ),
    crt!(
        "putc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::fputc
    ),
    crt!(
        "fputs",
        "UCRT for MinGW-built Windows libraries",
        ucrt::fputs
    ),
    crt!(
        "getc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::getc
    ),
    crt!(
        "fgetc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::getc
    ),
    crt!(
        "ungetc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::ungetc
    ),
    crt!(
        "putwc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::putwc
    ),
    crt!(
        "fputwc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::putwc
    ),
    crt!(
        "getwc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::getwc
    ),
    crt!(
        "fgetwc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::getwc
    ),
    crt!(
        "ungetwc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::ungetwc
    ),
    crt!(
        "setvbuf",
        "UCRT for MinGW-built Windows libraries",
        ucrt::setvbuf
    ),
    crt!(
        "_lock_file",
        "UCRT for MinGW-built Windows libraries",
        ucrt::lock_file
    ),
    crt!(
        "_unlock_file",
        "UCRT for MinGW-built Windows libraries",
        ucrt::unlock_file
    ),
    crt!(
        "_fdopen",
        "UCRT for MinGW-built Windows libraries",
        ucrt::fdopen
    ),
    crt!(
        "_lseeki64",
        "UCRT for MinGW-built Windows libraries",
        ucrt::lseeki64
    ),
    crt!(
        "_read",
        "UCRT for MinGW-built Windows libraries",
        ucrt::read
    ),
    crt!(
        "_write",
        "UCRT for MinGW-built Windows libraries",
        ucrt::write
    ),
    crt!(
        "_fstat64",
        "UCRT for MinGW-built Windows libraries",
        ucrt::fstat64
    ),
    crt!(
        "remove",
        "UCRT for MinGW-built Windows libraries",
        ucrt::remove
    ),
    crt!(
        "_initialize_onexit_table",
        "UCRT for MinGW-built Windows libraries",
        ucrt::initialize_onexit_table
    ),
    crt!(
        "_register_onexit_function",
        "UCRT for MinGW-built Windows libraries",
        ucrt::register_onexit_function
    ),
    crt!(
        "_execute_onexit_table",
        "UCRT for MinGW-built Windows libraries",
        ucrt::execute_onexit_table
    ),
    crt!(
        "_crt_atexit",
        "UCRT for MinGW-built Windows libraries",
        ucrt::crt_atexit
    ),
    crt!(
        "_crt_at_quick_exit",
        "UCRT for MinGW-built Windows libraries",
        ucrt::crt_at_quick_exit
    ),
    crt!(
        "_configure_narrow_argv",
        "UCRT for MinGW-built Windows libraries",
        ucrt::configure_argv
    ),
    crt!(
        "_configure_wide_argv",
        "UCRT for MinGW-built Windows libraries",
        ucrt::configure_argv
    ),
    crt!(
        "_initialize_narrow_environment",
        "UCRT for MinGW-built Windows libraries",
        ucrt::initialize_environment
    ),
    crt!(
        "_initialize_wide_environment",
        "UCRT for MinGW-built Windows libraries",
        ucrt::initialize_environment
    ),
    crt!(
        "__p___argc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::p_argc
    ),
    crt!(
        "__p___argv",
        "UCRT for MinGW-built Windows libraries",
        ucrt::p_argv
    ),
    crt!(
        "__p___wargv",
        "UCRT for MinGW-built Windows libraries",
        ucrt::p_wargv
    ),
    crt!(
        "__p__environ",
        "UCRT for MinGW-built Windows libraries",
        ucrt::p_environ
    ),
    crt!(
        "__p__wenviron",
        "UCRT for MinGW-built Windows libraries",
        ucrt::p_wenviron
    ),
    crt!(
        "_endthreadex",
        "UCRT for MinGW-built Windows libraries",
        ucrt::end_thread
    ),
    crt!(
        "_exit",
        "UCRT for MinGW-built Windows libraries",
        ucrt::exit_now
    ),
    crt!(
        "abort",
        "UCRT for MinGW-built Windows libraries",
        ucrt::abort
    ),
    crt!(
        "signal",
        "UCRT for MinGW-built Windows libraries",
        ucrt::signal
    ),
    crt!(
        "strerror",
        "UCRT for MinGW-built Windows libraries",
        ucrt::strerror
    ),
    crt!(
        "strtoul",
        "UCRT for MinGW-built Windows libraries",
        ucrt::strtoul
    ),
    crt!(
        "rand_s",
        "UCRT for MinGW-built Windows libraries",
        ucrt::rand_s
    ),
    crt!(
        "_aligned_malloc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::aligned_malloc
    ),
    crt!(
        "_aligned_free",
        "UCRT for MinGW-built Windows libraries",
        ucrt::aligned_free
    ),
    crt!(
        "_set_new_mode",
        "UCRT for MinGW-built Windows libraries",
        ucrt::set_new_mode
    ),
    crt!(
        "fmaxf",
        "UCRT for MinGW-built Windows libraries",
        ucrt::fmaxf
    ),
    crt!(
        "fminf",
        "UCRT for MinGW-built Windows libraries",
        ucrt::fminf
    ),
    crt!(
        "powf",
        "UCRT for MinGW-built Windows libraries",
        ucrt::powf
    ),
    crt!(
        "towlower",
        "UCRT for MinGW-built Windows libraries",
        ucrt::towlower
    ),
    crt!(
        "towupper",
        "UCRT for MinGW-built Windows libraries",
        ucrt::towupper
    ),
    crt!(
        "wcslen",
        "UCRT for MinGW-built Windows libraries",
        ucrt::wcslen
    ),
    crt!(
        "wcscmp",
        "UCRT for MinGW-built Windows libraries",
        ucrt::wcscmp
    ),
    crt!(
        "wcscoll",
        "UCRT for MinGW-built Windows libraries",
        ucrt::wcscmp
    ),
    crt!(
        "_wcsicmp",
        "UCRT for MinGW-built Windows libraries",
        ucrt::wcsicmp
    ),
    crt!(
        "wcsxfrm",
        "UCRT for MinGW-built Windows libraries",
        ucrt::wcsxfrm
    ),
    crt!(
        "strcoll",
        "UCRT for MinGW-built Windows libraries",
        ucrt::strcoll
    ),
    crt!(
        "strxfrm",
        "UCRT for MinGW-built Windows libraries",
        ucrt::strxfrm
    ),
    crt!(
        "iswctype",
        "UCRT for MinGW-built Windows libraries",
        ucrt::iswctype
    ),
    crt!(
        "mbrtowc",
        "UCRT for MinGW-built Windows libraries",
        ucrt::mbrtowc
    ),
    crt!(
        "wcrtomb",
        "UCRT for MinGW-built Windows libraries",
        ucrt::wcrtomb
    ),
    crt!(
        "localeconv",
        "UCRT for MinGW-built Windows libraries",
        ucrt::localeconv
    ),
    crt!(
        "strftime",
        "UCRT for MinGW-built Windows libraries",
        ucrt::strftime
    ),
    crt!(
        "wcsftime",
        "UCRT for MinGW-built Windows libraries",
        ucrt::wcsftime
    ),
    crt!(
        "_tzset",
        "UCRT for MinGW-built Windows libraries",
        ucrt::tzset
    ),
    crt!(
        "__daylight",
        "UCRT for MinGW-built Windows libraries",
        ucrt::p_daylight
    ),
    crt!(
        "__timezone",
        "UCRT for MinGW-built Windows libraries",
        ucrt::p_timezone
    ),
    crt!(
        "__tzname",
        "UCRT for MinGW-built Windows libraries",
        ucrt::p_tzname
    ),
    crt!(
        "__intrinsic_setjmpex",
        "UCRT for MinGW-built Windows libraries",
        ucrt::setjmp
    ),
    crt!(
        "_setjmp",
        "UCRT for MinGW-built Windows libraries",
        ucrt::setjmp
    ),
    crt!(
        "longjmp",
        "UCRT for MinGW-built Windows libraries",
        ucrt::longjmp
    ),
    crt!(
        "fopen",
        "registered Windows FILE wrappers over binary host streams",
        stdio::open
    ),
    crt!(
        "_wfopen",
        "registered Windows FILE wrappers over binary host streams",
        stdio::open_w
    ),
    crt!(
        "_wfopen_s",
        "registered Windows FILE wrappers over binary host streams",
        stdio::open_w_s
    ),
    crt!(
        "_wfsopen",
        "registered Windows FILE wrappers over binary host streams",
        stdio::shared_open_w
    ),
    crt!(
        "fclose",
        "registered Windows FILE wrappers over binary host streams",
        stdio::close
    ),
    crt!(
        "fread",
        "registered Windows FILE wrappers over binary host streams",
        stdio::read
    ),
    crt!(
        "fwrite",
        "registered Windows FILE wrappers over binary host streams",
        stdio::write
    ),
    crt!(
        "fseek",
        "registered Windows FILE wrappers over binary host streams",
        stdio::seek
    ),
    crt!(
        "_fseeki64",
        "registered Windows FILE wrappers over binary host streams",
        stdio::seek64
    ),
    crt!(
        "ftell",
        "registered Windows FILE wrappers over binary host streams",
        stdio::tell
    ),
    crt!(
        "_ftelli64",
        "registered Windows FILE wrappers over binary host streams",
        stdio::tell64
    ),
    crt!(
        "ferror",
        "registered Windows FILE wrappers over binary host streams",
        stdio::file_error
    ),
    crt!(
        "feof",
        "registered Windows FILE wrappers over binary host streams",
        stdio::eof
    ),
    crt!(
        "fgetpos",
        "registered Windows FILE wrappers over binary host streams",
        stdio::getpos
    ),
    crt!(
        "fsetpos",
        "registered Windows FILE wrappers over binary host streams",
        stdio::setpos
    ),
    crt!(
        "_errno",
        "registered Windows FILE wrappers over binary host streams",
        stdio::errno
    ),
    crt!(
        "getenv",
        "inherited host environment byte strings",
        simple::environment
    ),
    crt!(
        "___lc_codepage_func",
        "CRT C locale; codepage zero",
        locale::codepage
    ),
    crt!(
        "___mb_cur_max_func",
        "CRT C locale; single-byte characters",
        locale::max_bytes
    ),
    crt!(
        "setlocale",
        "supports C/POSIX locale only; unsupported locales fail",
        locale::set_locale
    ),
    crt!(
        "_beginthreadex",
        "host worker with Windows ABI/TLS and waitable thread handle",
        crate::api::threading::threads::begin_thread
    ),
    crt!(
        "__iob_func",
        "Windows FILE layouts for inherited standard streams",
        stdio::iob
    ),
    crt!(
        "setbuf",
        "standard streams; supports unbuffered mode",
        stdio::set_buffer
    ),
    crt!("fflush", "registered standard streams", stdio::flush),
    crt!("_fileno", "registered standard streams", stdio::descriptor),
    crt!(
        "_get_osfhandle",
        "inherited standard descriptors only",
        stdio::os_handle
    ),
    crt!(
        "malloc",
        "host memory/byte-string ABI wrapper",
        simple::call_0
    ),
    crt!(
        "free",
        "host memory/byte-string ABI wrapper",
        simple::call_1
    ),
    crt!(
        "calloc",
        "host memory/byte-string ABI wrapper",
        simple::call_2
    ),
    crt!(
        "realloc",
        "host memory/byte-string ABI wrapper",
        simple::call_3
    ),
    crt!(
        "memcpy",
        "host memory/byte-string ABI wrapper",
        simple::call_4
    ),
    crt!(
        "memmove",
        "host memory/byte-string ABI wrapper",
        simple::call_5
    ),
    crt!(
        "memset",
        "host memory/byte-string ABI wrapper",
        simple::call_6
    ),
    crt!(
        "memcmp",
        "host memory/byte-string ABI wrapper",
        simple::call_7
    ),
    crt!(
        "memchr",
        "host memory/byte-string ABI wrapper",
        simple::call_8
    ),
    crt!(
        "strlen",
        "host memory/byte-string ABI wrapper",
        simple::call_9
    ),
    crt!(
        "strcmp",
        "host memory/byte-string ABI wrapper",
        simple::call_10
    ),
    crt!(
        "strncmp",
        "host memory/byte-string ABI wrapper",
        simple::call_11
    ),
    crt!(
        "strcpy",
        "host memory/byte-string ABI wrapper",
        simple::call_12
    ),
    crt!(
        "strncpy",
        "host memory/byte-string ABI wrapper",
        simple::call_13
    ),
    crt!(
        "strcat",
        "host memory/byte-string ABI wrapper",
        simple::call_14
    ),
    crt!(
        "strncat",
        "host memory/byte-string ABI wrapper",
        simple::call_15
    ),
    crt!(
        "strchr",
        "host memory/byte-string ABI wrapper",
        simple::call_16
    ),
    crt!(
        "strrchr",
        "host memory/byte-string ABI wrapper",
        simple::call_17
    ),
    crt!(
        "strstr",
        "host memory/byte-string ABI wrapper",
        simple::call_18
    ),
    crt!(
        "strpbrk",
        "host memory/byte-string ABI wrapper",
        simple::call_19
    ),
    crt!(
        "strcspn",
        "host memory/byte-string ABI wrapper",
        simple::call_20
    ),
    crt!(
        "_strdup",
        "host memory/byte-string ABI wrapper",
        simple::call_21
    ),
    crt!(
        "atoi",
        "host memory/byte-string ABI wrapper",
        simple::call_22
    ),
    crt!(
        "atof",
        "host memory/byte-string ABI wrapper",
        simple::call_23
    ),
    crt!(
        "atexit",
        "Windows callbacks executed in reverse order at process exit",
        startup::register
    ),
    crt!(
        "_onexit",
        "Windows callbacks executed in reverse order",
        startup::on_exit
    ),
    crt!(
        "_initterm",
        "calls Windows initializer array",
        startup::init_term
    ),
    crt!(
        "_cexit",
        "runs registered callbacks and flushes host streams",
        startup::cexit
    ),
    crt!(
        "exit",
        "terminates process after registered callbacks",
        startup::exit
    ),
    crt!(
        "_amsg_exit",
        "reports runtime startup failure",
        startup::amsg_exit
    ),
    crt!(
        "__set_app_type",
        "records console/GUI CRT application type",
        startup::app_type
    ),
    crt!(
        "__setusermatherr",
        "records custom CRT math handler; math wrappers pending",
        startup::matherr
    ),
    crt!(
        "__getmainargs",
        "narrow command line and environment; wildcard expansion unsupported",
        startup::mainargs
    ),
    data!(
        "msvcrt.dll",
        "_fmode",
        "crt",
        "partial",
        "CRT default file mode variable",
        startup::FMODE
    ),
    data!(
        "msvcrt.dll",
        "_commode",
        "crt",
        "partial",
        "CRT commit mode variable",
        startup::COMMODE
    ),
    data!(
        "msvcrt.dll",
        "__initenv",
        "crt",
        "partial",
        "environment pointer initialized by __getmainargs",
        startup::CRT_INITENV
    ),
}
