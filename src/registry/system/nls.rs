//! Export declarations for nls. Implementation stays in api/.
use crate::api::system::encoding;
use crate::api::system::formats;
use crate::api::system::locale;
use crate::registry::Export;

exports! {
    implemented!("CompareStringA", "system", "partial", "ANSI (UTF-8) decoded to wide; ordinal order, optional case folding", locale::compare_a),

    implemented!("GetThreadLocale", "system", "partial", "calling-thread LCID, inherited at CreateThread; host locale default", locale::thread::get),
    implemented!("SetThreadLocale", "system", "partial", "known LCID/invariant thread state and TEB; custom locales unsupported; UTF-8 ACP policy unchanged", locale::thread::set),
    implemented!("GetCPInfoExW", "system", "partial", "UTF-8 and Windows-1252 metadata", locale::cp_info_ex_w),
    implemented!(
        "GetDateFormatW",
        "system",
        "partial",
        "format pictures; English names, en-US defaults",
        formats::date_format_w
    ),
    implemented!(
        "GetDateFormatEx",
        "system",
        "partial",
        "format pictures; English names, en-US defaults",
        formats::date_format_ex
    ),
    implemented!(
        "GetTimeFormatW",
        "system",
        "partial",
        "format pictures; en-US default",
        formats::time_format_w
    ),
    implemented!(
        "GetTimeFormatEx",
        "system",
        "partial",
        "format pictures; en-US default",
        formats::time_format_ex
    ),
    implemented!(
        "LocaleNameToLCID",
        "system",
        "partial",
        "user's locale and known language tags",
        locale::locale_name_to_lcid
    ),
    implemented!(
        "LCIDToLocaleName",
        "system",
        "partial",
        "user's locale and known language tags",
        locale::lcid_to_locale_name
    ),
    implemented!(
        "IsValidLocaleName",
        "system",
        "partial",
        "user's locale and known language tags",
        locale::is_valid_locale_name
    ),
    implemented!(
        "EnumSystemLocalesEx",
        "system",
        "partial",
        "the user's locale",
        locale::enum_system_locales_ex
    ),
    implemented!(
        "GetLocaleInfoW",
        "system",
        "partial",
        "user locale only; name, language, country, separators, measure",
        locale::info_w
    ),
    implemented!(
        "GetLocaleInfoEx",
        "system",
        "partial",
        "user locale only; name, language, country, separators, measure",
        locale::info_ex
    ),
    implemented!(
        "GetUserDefaultLCID",
        "system",
        "implemented",
        "LANGID of the host locale",
        locale::user_default_lcid
    ),
    implemented!(
        "GetUserDefaultLocaleName",
        "system",
        "implemented",
        "host locale as a Windows name",
        locale::user_default_locale_name
    ),
    implemented!(
        "GetSystemDefaultLocaleName",
        "system",
        "implemented",
        "host locale as a Windows name",
        locale::user_default_locale_name
    ),
    implemented!(
        "IsValidLocale",
        "system",
        "partial",
        "the user locale and en-US",
        locale::is_valid_locale
    ),
    implemented!(
        "EnumSystemLocalesW",
        "system",
        "partial",
        "the user locale only",
        locale::enum_system_locales_w
    ),
    implemented!(
        "IsValidCodePage",
        "system",
        "partial",
        "UTF-8 and 1252",
        locale::is_valid_code_page
    ),
    implemented!(
        "GetCPInfo",
        "system",
        "partial",
        "UTF-8 and 1252",
        locale::cp_info
    ),
    implemented!(
        "GetConsoleOutputCP",
        "system",
        "implemented",
        "UTF-8 terminal",
        locale::console_output_cp
    ),
    implemented!(
        "GetConsoleCP",
        "system",
        "implemented",
        "UTF-8 terminal",
        locale::console_output_cp
    ),
    implemented!(
        "GetStringTypeW",
        "system",
        "partial",
        "CT_CTYPE1 from Unicode properties",
        locale::string_type_w
    ),
    implemented!(
        "GetStringTypeA",
        "system",
        "partial",
        "CT_CTYPE1 for narrow byte strings",
        locale::string_type_a
    ),
    implemented!(
        "GetLocaleInfoA",
        "system",
        "partial",
        "ANSI wrapper over GetLocaleInfoW",
        locale::info_a
    ),
    implemented!(
        "LCMapStringW",
        "system",
        "partial",
        "upper/lower case mapping only",
        locale::lcmap_w
    ),
    implemented!(
        "LCMapStringA",
        "system",
        "partial",
        "upper/lower case mapping for narrow byte strings",
        locale::lcmap_a
    ),
    implemented!(
        "LCMapStringEx",
        "system",
        "partial",
        "upper/lower case mapping only",
        locale::lcmap_ex
    ),
    implemented!(
        "CompareStringW",
        "system",
        "partial",
        "ordinal UTF-16 order, optional case folding",
        locale::compare_w
    ),
    implemented!(
        "CompareStringEx",
        "system",
        "partial",
        "ordinal UTF-16 order, optional case folding",
        locale::compare_ex
    ),
    implemented!(
        "GetACP",
        "system",
        "partial",
        "UTF-8 ANSI policy",
        encoding::codepage
    ),
    implemented!(
        "GetOEMCP",
        "system",
        "partial",
        "UTF-8 OEM policy",
        encoding::codepage
    ),
    implemented!(
        "WideCharToMultiByte",
        "system",
        "partial",
        "UTF-8/ANSI UTF-16 conversion; validates flags and buffers",
        encoding::wide_to_multi
    ),
    implemented!(
        "MultiByteToWideChar",
        "system",
        "partial",
        "UTF-8/ANSI UTF-16 conversion; validates flags and buffers",
        encoding::multi_to_wide
    ),
    implemented!("GetUserDefaultUILanguage", "system", "partial", "LC_ALL, LC_MESSAGES, LANG mapped to Windows LANGID; C/unknown fallback en-US", locale::user_default_ui_language),
    implemented!("GetUserPreferredUILanguages", "system", "partial", "user preferred UI languages from host locale", locale::preferred_ui_languages),
    implemented!("GetThreadPreferredUILanguages", "system", "partial", "thread-only empty list or merged host UI language", locale::thread_preferred_ui_languages),
    implemented!("GetSystemPreferredUILanguages", "system", "partial", "system preferred UI languages from host locale", locale::preferred_ui_languages),
}
