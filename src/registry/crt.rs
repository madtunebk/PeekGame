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
    crt!("_get_stream_buffer_pointers", "pointers into the FILE layout; host-buffered streams report an empty buffer", crate::api::crt::cxx_support::stream_buffer_pointers),
    crt!("_lock_locales", "recursive global locale lock", crate::api::crt::cxx_support::lock_locales),
    crt!("_unlock_locales", "recursive global locale lock", crate::api::crt::cxx_support::unlock_locales),
    crt!("___lc_locale_name_func", "C locale: NULL names per category", crate::api::crt::cxx_support::locale_names),
    crt!("___lc_collate_cp_func", "C locale collation code page 0", crate::api::crt::cxx_support::collate_codepage),
    crt!("__pctype_func", "C locale ctype table (-1..255)", crate::api::crt::cxx_support::ctype_table),
    crt!("_wsetlocale", "C/POSIX/native(C) only; others fail", crate::api::crt::cxx_support::wsetlocale),
    crt!("isupper", "C locale ASCII", crate::api::crt::cxx_support::is_upper),
    crt!("islower", "C locale ASCII", crate::api::crt::cxx_support::is_lower),
    crt!("__strncnt", "strnlen", crate::api::crt::cxx_support::strncnt),
    crt!("strnlen", "bounded narrow string length", crate::api::crt::cxx_support::strncnt),
    crt!("wcsnlen", "bounded wide length", crate::api::crt::cxx_support::wcsnlen),
    crt!("_wcsdup", "host malloc copy (free compatible)", crate::api::crt::cxx_support::wcsdup),
    crt!("strtod", "host strtod, C locale", crate::api::crt::cxx_support::str_to_double),
    crt!("strtof", "host strtof, C locale", crate::api::crt::cxx_support::str_to_float),
    crt!("wcstod", "ASCII prefix through strtod", crate::api::crt::cxx_support::wide_to_double),
    crt!("wcstof", "ASCII prefix through strtof", crate::api::crt::cxx_support::wide_to_float),
    crt!("atol", "32-bit Windows long", crate::api::crt::cxx_support::atol),
    crt!("_set_new_handler", "stores the C++ new handler", crate::api::crt::cxx_support::set_new_handler),
    crt!("_callnewh", "runs the new handler after a failed allocation", crate::api::crt::cxx_support::call_new_handler),
    crt!("_set_invalid_parameter_handler", "sets global invalid parameter handler", crate::api::crt::cxx_support::set_invalid_parameter_handler),
    crt!("_get_invalid_parameter_handler", "gets global invalid parameter handler", crate::api::crt::cxx_support::get_invalid_parameter_handler),
    crt!("_set_thread_local_invalid_parameter_handler", "sets thread invalid parameter handler", crate::api::crt::cxx_support::set_thread_local_invalid_parameter_handler),
    crt!("_get_thread_local_invalid_parameter_handler", "gets thread invalid parameter handler", crate::api::crt::cxx_support::get_thread_local_invalid_parameter_handler),
    crt!("_invalid_parameter", "invokes invalid parameter handler", crate::api::crt::cxx_support::invalid_parameter),
    crt!("_invalid_parameter_noinfo", "invokes invalid parameter handler without info", crate::api::crt::cxx_support::invalid_parameter_noinfo),
    crt!("_invalid_parameter_noinfo_noreturn", "invokes invalid parameter handler and aborts", crate::api::crt::cxx_support::invalid_parameter_noinfo_noreturn),
    crt!("_seh_filter_dll", "EXCEPTION_CONTINUE_SEARCH", crate::api::crt::cxx_support::seh_filter_dll),
    crt!("__stdio_common_vsprintf_s", "truncation empties the buffer, ERANGE, -1", crate::api::crt::cxx_support::common_vsprintf_s),
    crt!("vsprintf", "standard vsprintf", crate::api::crt::cxx_support::vsprintf),
    crt!("_dclass", "FP_* classification", crate::api::crt::cxx_support::dclass),
    crt!("_fdclass", "FP_* classification", crate::api::crt::cxx_support::fdclass),
    crt!("_dsign", "sign bit", crate::api::crt::cxx_support::dsign),
    crt!("_fdsign", "sign bit", crate::api::crt::cxx_support::fdsign),
    crt!("copysign", "IEEE copysign", crate::api::crt::cxx_support::copy_sign),
    crt!("frexp", "mantissa/exponent incl. subnormals", crate::api::crt::cxx_support::frexp),
    crt!("lgamma", "host lgamma", crate::api::crt::cxx_support::log_gamma),
    crt!("lgammal", "host lgamma (long double is double)", crate::api::crt::cxx_support::log_gamma),
    crt!("lgammaf", "host lgammaf", crate::api::crt::cxx_support::log_gamma_f),
    crt!("_wremove", "DeleteFileW with errno", crate::api::crt::cxx_support::wremove),
    crt!("_wrename", "MoveFileW with errno", crate::api::crt::cxx_support::wrename),
    crt!("_wrmdir", "RemoveDirectoryW with errno", crate::api::crt::cxx_support::wrmdir),
    crt!("_fsopen", "fopen; share flag ignored", crate::api::crt::cxx_support::fsopen),
    crt!("_get_initial_narrow_environment", "stable initial CRT UTF-8 environment snapshot", startup::initial_narrow_environment),
    crt!("_get_narrow_winmain_command_line", "WinMain narrow command line", startup::get_narrow_winmain_command_line),
    crt!("_get_wide_winmain_command_line", "WinMain wide command line", startup::get_wide_winmain_command_line),
    crt!("trunc", "typed Win64 libm adapter; CRT math errno/fenv parity incomplete", crate::api::crt::math::trunc),
    crt!("round", "typed Win64 libm adapter; CRT math errno/fenv parity incomplete", crate::api::crt::math::round),
    crt!("truncf", "typed Win64 libm adapter; CRT math errno/fenv parity incomplete", crate::api::crt::math::truncf),
    crt!("roundf", "typed Win64 libm adapter; CRT math errno/fenv parity incomplete", crate::api::crt::math::roundf),
    crt!("exp2f", "typed Win64 libm adapter; CRT math errno/fenv parity incomplete", crate::api::crt::math::exp2f),
    crt!("exp2", "typed Win64 libm adapter; CRT math errno/fenv parity incomplete", crate::api::crt::math::exp2),

    crt!("_configthreadlocale", "thread-local query/enable/disable policy; C/POSIX locales only", locale::configure_thread),
    crt!("__p__commode", "shared CRT commit-mode storage; automatic commit-on-flush unsupported", crate::api::crt::file_mode::commode_pointer),
    crt!("_set_fmode", "set CRT default translation mode; stream translation remains partial", crate::api::crt::file_mode::set),
    crt!("_get_fmode", "read CRT default file mode; null output fails EINVAL", crate::api::crt::file_mode::get),
    crt!("__p__fmode", "pointer to shared CRT default file mode", crate::api::crt::file_mode::pointer),
    crt!("_set_app_type", "UCRT startup: store unknown/console/GUI application type", startup::app_type),
    crt!("_query_app_type", "read process CRT application type", startup::query_app_type),
    crt!("_get_heap_handle", "CRT heap handle (process heap)", crate::api::crt::cxx_support::get_heap_handle),
    crt!("acos", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::acos),
    crt!("acosf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::acosf),
    crt!("asin", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::asin),
    crt!("asinf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::asinf),
    crt!("atan", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::atan),
    crt!("atanf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::atanf),
    crt!("ceil", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::ceil),
    crt!("ceilf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::ceilf),
    crt!("cos", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::cos),
    crt!("cosf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::cosf),
    crt!("cosh", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::cosh),
    crt!("coshf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::coshf),
    crt!("exp", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::exp),
    crt!("expf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::expf),
    crt!("floor", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::floor),
    crt!("floorf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::floorf),
    crt!("log", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::log),
    crt!("logf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::logf),
    crt!("log10", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::log10),
    crt!("log10f", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::log10f),
    crt!("sin", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::sin),
    crt!("sinf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::sinf),
    crt!("sinh", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::sinh),
    crt!("sinhf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::sinhf),
    crt!("tan", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::tan),
    crt!("tanh", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::tanh),
    crt!("atan2", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::atan2),
    crt!("atan2f", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::atan2f),
    crt!("fmod", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::fmod),
    crt!("fmodf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::fmodf),
    crt!("pow", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::pow),
    crt!("_hypot", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::hypot),
    crt!("_hypotf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::hypotf),
    crt!("cbrtf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::cbrtf),
    crt!("fma", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::fma),
    crt!("fmaf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::fmaf),
    crt!("log1p", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::log1p),
    crt!("log1pf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::log1pf),
    crt!("log2", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::log2),
    crt!("log2f", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::log2f),
    crt!("_nextafter", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::nextafter),
    crt!("ldexp", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::ldexp),
    crt!("tanf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::tanf),
    crt!("tanhf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::math::tanhf),
    crt!("qsort", "ported CRT implementation; Windows x64 ABI", crate::api::crt::algorithms::qsort),
    crt!("bsearch", "ported CRT implementation; Windows x64 ABI", crate::api::crt::algorithms::bsearch),
    crt!("isalnum", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::isalnum),
    crt!("isalpha", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::isalpha),
    crt!("isdigit", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::isdigit),
    crt!("isspace", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::isspace),
    crt!("isxdigit", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::isxdigit),
    crt!("iswalnum", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::iswalnum),
    crt!("iswalpha", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::iswalpha),
    crt!("iswdigit", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::iswdigit),
    crt!("iswxdigit", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::iswxdigit),
    crt!("iswspace", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::iswspace),
    crt!("tolower", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::tolower),
    crt!("toupper", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::toupper),
    crt!("wcscpy", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::wcscpy),
    crt!("strcpy_s", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::strcpy_s),
    crt!("strcat_s", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::strcat_s),
    crt!("wcscpy_s", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::wcscpy_s),
    crt!("_memicmp", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::memicmp),
    crt!("_stricmp", "ported CRT implementation; Windows x64 ABI", crate::api::crt::strings::stricmp),
    crt!("strtol", "ported CRT implementation; Windows x64 ABI", crate::api::crt::numbers::strtol),
    crt!("wcstol", "ported CRT implementation; Windows x64 ABI", crate::api::crt::numbers::wcstol),
    crt!("rand", "ported CRT implementation; Windows x64 ABI", crate::api::crt::numbers::rand),
    crt!("srand", "ported CRT implementation; Windows x64 ABI", crate::api::crt::numbers::srand),
    crt!("_strtoi64", "ported CRT implementation; Windows x64 ABI", crate::api::crt::numbers::strtoi64),
    crt!("_strtoui64", "ported CRT implementation; Windows x64 ABI", crate::api::crt::numbers::strtoui64),
    crt!("_itoa_s", "ported CRT implementation; Windows x64 ABI", crate::api::crt::numbers::itoa_s),
    crt!("mbstowcs", "ported CRT implementation; Windows x64 ABI", crate::api::crt::encoding::mbstowcs),
    crt!("_lock", "ported CRT implementation; Windows x64 ABI", crate::api::crt::control::lock),
    crt!("_unlock", "ported CRT implementation; Windows x64 ABI", crate::api::crt::control::unlock),
    crt!("_set_error_mode", "ported CRT implementation; Windows x64 ABI", crate::api::crt::control::set_error_mode),
    crt!("_initterm_e", "ported CRT implementation; Windows x64 ABI", crate::api::crt::control::init_term_e),
    crt!("_getpid", "ported CRT implementation; Windows x64 ABI", crate::api::crt::control::getpid),
    crt!("_fpreset", "ported CRT implementation; Windows x64 ABI", crate::api::crt::control::fpreset),
    crt!("_assert", "ported CRT implementation; Windows x64 ABI", crate::api::crt::control::assertion),
    crt!("_wgetcwd", "ported CRT implementation; Windows x64 ABI", crate::api::crt::directory::getcwd_w),
    crt!("clock", "ported CRT implementation; Windows x64 ABI", crate::api::crt::time::clock),
    crt!("_time64", "ported CRT implementation; Windows x64 ABI", crate::api::crt::time::time64),
    crt!("_gmtime64", "ported CRT implementation; Windows x64 ABI", crate::api::crt::time::gmtime64),
    crt!("_gmtime64_s", "ported CRT implementation; Windows x64 ABI", crate::api::crt::time::gmtime64_s),
    crt!("fgets", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::fgets),
    crt!("rewind", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::rewind),
    crt!("puts", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::puts),
    crt!("_cputs", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::cputs),
    crt!("putchar", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::putchar),
    crt!("perror", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::perror),
    crt!("freopen_s", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::freopen_s),
    crt!("_chsize", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::chsize),
    crt!("_fstat", "ported CRT implementation; Windows x64 ABI", crate::api::crt::streams::fstat),
    crt!("vfprintf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::format::vfprintf),
    crt!("fprintf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::format::fprintf),
    crt!("fwprintf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::format::fwprintf),
    crt!("_vsnprintf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::format::vsnprintf),
    crt!("_vsnprintf_s", "ported CRT implementation; Windows x64 ABI", crate::api::crt::format::vsnprintf_s),
    crt!("_vscprintf", "ported CRT implementation; Windows x64 ABI", crate::api::crt::format::vscprintf),
    data!("msvcrt.dll", "_timezone", "crt", "partial", "Windows 32-bit timezone seconds", ucrt::CRT_TIMEZONE),
    data!("msvcrt.dll", "_tzname", "crt", "partial", "timezone name pointers updated by _tzset", ucrt::CRT_TZNAME),
    crt!("wcstombs", "UTF-16 to UTF-8; byte capacity, size query and EILSEQ validation", crate::api::crt::encoding::wcstombs),

    crt!("_filelengthi64", "64-bit descriptor file size via fstat; preserves seek position", ucrt::filelengthi64),
    crt!("_wchdir", "UTF-16 current directory using shared Windows path mapping; CRT errno on failure", crate::api::crt::directory::change_w),
    crt!("_setmaxstdio", "sets and enforces the process CRT stream limit (20..8192)", stdio::set_max_stdio),
    crt!("_getmaxstdio", "current CRT stream limit", stdio::get_max_stdio),
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
        "_c_exit",
        "performs quick cleanup without running atexit callbacks",
        startup::c_exit
    ),
    crt!(
        "terminate",
        "aborts host process",
        ucrt::terminate
    ),
    crt!(
        "_seh_filter_exe",
        "default unhandled exception filter for EXE; continues search",
        startup::seh_filter_exe
    ),
    crt!(
        "_register_thread_local_exe_atexit_callback",
        "registers thread-local exit callback with host process runtime",
        startup::register_thread_local_exe_atexit_callback
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
