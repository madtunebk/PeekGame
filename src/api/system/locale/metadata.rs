//! Metadata for the LANGIDs recognized by the runtime.
use super::langid;
/// Legacy locale metadata, distinct from the process ANSI code page (UTF-8).
pub(super) fn ansi_code_page() -> u32 {
    let id = langid();
    match id & 0x3ff {
        0x04 => {
            if matches!(id, 0x0404 | 0x0c04) {
                950
            } else {
                936
            }
        }
        0x11 => 932,
        0x12 => 949,
        0x01 => 1256,
        0x0d => 1255,
        0x1e => 874,
        0x2a => 1258,
        0x08 => 1253,
        0x1f => 1254,
        0x25 | 0x26 | 0x27 => 1257,
        0x02 | 0x19 | 0x22 => 1251,
        0x05 | 0x0e | 0x15 | 0x18 | 0x1a | 0x1b | 0x24 => 1250,
        0x39 => 0, // Hindi is a Unicode-only Windows locale.
        _ => 1252, // Remaining languages in TABLE use the Western code page.
    }
}
pub(super) fn english_language() -> &'static str {
    match langid() & 0x3ff {
        0x01 => "Arabic",
        0x02 => "Bulgarian",
        0x03 => "Catalan",
        0x04 => "Chinese",
        0x05 => "Czech",
        0x06 => "Danish",
        0x07 => "German",
        0x08 => "Greek",
        0x09 => "English",
        0x0a => "Spanish",
        0x0b => "Finnish",
        0x0c => "French",
        0x0d => "Hebrew",
        0x0e => "Hungarian",
        0x0f => "Icelandic",
        0x10 => "Italian",
        0x11 => "Japanese",
        0x12 => "Korean",
        0x13 => "Dutch",
        0x14 => "Norwegian",
        0x15 => "Polish",
        0x16 => "Portuguese",
        0x18 => "Romanian",
        0x19 => "Russian",
        0x1a => {
            if langid() == 0x081a {
                "Serbian"
            } else {
                "Croatian"
            }
        }
        0x1b => "Slovak",
        0x1d => "Swedish",
        0x1e => "Thai",
        0x1f => "Turkish",
        0x21 => "Indonesian",
        0x22 => "Ukrainian",
        0x24 => "Slovenian",
        0x25 => "Estonian",
        0x26 => "Latvian",
        0x27 => "Lithuanian",
        0x2a => "Vietnamese",
        0x39 => "Hindi",
        _ => unreachable!("langid table and English metadata must agree"),
    }
}
pub(super) fn english_country(country: &str) -> Option<&'static str> {
    Some(match country {
        "" => "",
        "US" => "United States",
        "GB" => "United Kingdom",
        "AU" => "Australia",
        "CA" => "Canada",
        "NZ" => "New Zealand",
        "IE" => "Ireland",
        "RO" => "Romania",
        "JP" => "Japan",
        "FR" => "France",
        "DE" => "Germany",
        "AT" => "Austria",
        "CH" => "Switzerland",
        "IT" => "Italy",
        "ES" => "Spain",
        "MX" => "Mexico",
        "PT" => "Portugal",
        "BR" => "Brazil",
        "CN" => "China",
        "TW" => "Taiwan",
        "HK" => "Hong Kong SAR",
        "SG" => "Singapore",
        "KR" => "Korea",
        "RU" => "Russia",
        "UA" => "Ukraine",
        "PL" => "Poland",
        "CZ" => "Czechia",
        "SK" => "Slovakia",
        "HU" => "Hungary",
        "BG" => "Bulgaria",
        "GR" => "Greece",
        "TR" => "Turkey",
        "NL" => "Netherlands",
        "BE" => "Belgium",
        "SE" => "Sweden",
        "DK" => "Denmark",
        "NO" => "Norway",
        "FI" => "Finland",
        "IL" => "Israel",
        "SA" => "Saudi Arabia",
        "EG" => "Egypt",
        "AE" => "United Arab Emirates",
        "HR" => "Croatia",
        "RS" => "Serbia",
        "SI" => "Slovenia",
        "LT" => "Lithuania",
        "LV" => "Latvia",
        "EE" => "Estonia",
        "VN" => "Vietnam",
        "TH" => "Thailand",
        "ID" => "Indonesia",
        "IN" => "India",
        _ => return None,
    })
}
