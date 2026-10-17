/* ICU date fields adapted to Windows date/time pattern syntax. */
#include <unicode/udat.h>
#include <unicode/udatpg.h>
#include <stdint.h>
#include <string.h>
static int pattern_for(const char *locale, uint32_t kind, UChar *pattern, UErrorCode *error) {
    if (kind == 0x78 || kind == 0x1006) {
        UDateTimePatternGenerator *generator = udatpg_open(locale, error);
        if (U_FAILURE(*error) || !generator) return -1;
        UChar ym[] = {'y','M','M','M','M'}, md[] = {'M','M','M','M','d'};
        int count = udatpg_getBestPattern(generator, kind == 0x78 ? md : ym, 5, pattern, 256, error);
        udatpg_close(generator); return count;
    }
    int time = kind == 0x1003 || kind == 0x79 || kind == 0x1e || kind == 0x23 || kind == 0x25 || kind == 0x1005;
    UDateFormatStyle date_style = kind == 0x20 || kind == 0x22 ? UDAT_FULL : UDAT_SHORT;
    UDateFormatStyle time_style = kind == 0x79 ? UDAT_SHORT : UDAT_MEDIUM;
    UDateFormat *format = udat_open(time ? time_style : UDAT_NONE, time ? UDAT_NONE : date_style, locale, NULL, 0, NULL, 0, error);
    if (U_FAILURE(*error) || !format) return -1;
    int count = udat_toPattern(format, 0, pattern, 256, error);
    udat_close(format); return count;
}
int32_t peekgame_nls_date_pattern(const char *locale, uint32_t kind, UChar *out, int32_t capacity) {
    UErrorCode error = U_ZERO_ERROR; UChar pattern[256];
    int length = pattern_for(locale, kind, pattern, &error);
    if (U_FAILURE(error) || length < 0 || length >= 256) return -1;
    int quoted = 0, count = 0, first_date = -1, year = 0, day = 0, month = 0, hour = 0, hour24 = 0, hour_at = -1, am_at = -1;
    for (int i = 0; i < length;) {
        UChar ch = pattern[i];
        if (ch == 0x27) { quoted = !quoted; if (count >= capacity) return -1; out[count++] = ch; ++i; continue; }
        int run = 1; while (i + run < length && pattern[i + run] == ch) ++run;
        UChar mapped = ch; int copies = run;
        if (!quoted && ((ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z'))) {
            switch (ch) {
            case 'y': case 'u': year = run == 1 ? 4 : run; if (first_date < 0) first_date = 2; mapped = 'y'; copies = run == 1 ? 4 : run; break;
            case 'M': case 'L': month = run; if (first_date < 0) first_date = 0; mapped = 'M'; break;
            case 'd': day = run; if (first_date < 0) first_date = 1; break;
            case 'E': case 'e': case 'c': mapped = 'd'; copies = run <= 3 ? 3 : 4; break;
            case 'a': case 'b': case 'B': mapped = 't'; copies = 2; am_at = i; break;
            case 'H': case 'k': mapped = 'H'; hour = run; hour24 = 1; hour_at = i; break;
            case 'h': case 'K': mapped = 'h'; hour = run; hour_at = i; break;
            case 'm': case 's': break;
            case 'G': mapped = 'g'; copies = 2; break;
            default: return -1;
            }
        }
        if (count + copies > capacity) return -1;
        for (int j = 0; j < copies; ++j) out[count++] = mapped;
        i += run;
    }
    int value = -1;
    switch (kind) {
    case 0x21: case 0x22: value = first_date; break;
    case 0x23: value = hour_at >= 0 ? hour24 : -1; break;
    case 0x24: value = year > 0 ? year >= 4 : -1; break;
    case 0x25: value = hour_at >= 0 ? hour >= 2 : -1; break;
    case 0x26: value = day > 0 ? day >= 2 : -1; break;
    case 0x27: value = month > 0 ? month >= 2 : -1; break;
    case 0x1005: value = am_at >= 0 && hour_at >= 0 && am_at < hour_at; break;
    case 0x1d: case 0x1e: {
        int q = 0, seen_field = 0;
        for (int i = 0; i < length; ++i) {
            UChar ch = pattern[i]; if (ch == 0x27) { q = !q; continue; }
            if (!q && ((ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z'))) { seen_field = 1; continue; }
            if (seen_field && (ch == '/' || ch == '.' || ch == ':' || ch == '-')) { out[0] = ch; return 1; }
        }
        return -1;
    }
    default: return count;
    }
    if (value < 0 || value > 9 || capacity < 1) return -1;
    out[0] = (UChar)('0' + value); return 1;
}
