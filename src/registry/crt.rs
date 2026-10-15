use super::Export;
use crate::api::crt::locale;
use crate::api::crt::stdio;
use crate::api::crt::{simple, startup};
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
