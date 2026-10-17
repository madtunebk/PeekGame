/* Calendar names from ICU; Windows weekdays start on Monday, ICU on Sunday. */
#include <unicode/udat.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>
int32_t peekgame_nls_calendar(const char *locale, uint32_t kind, UChar *out, int32_t capacity) {
    UErrorCode error = U_ZERO_ERROR;
    UDateFormat *format = udat_open(UDAT_DEFAULT, UDAT_DEFAULT, locale, NULL, 0, NULL, 0, &error);
    if (U_FAILURE(error) || !format) return -1;
    UDateFormatSymbolType type; int32_t index, result = -1;
    if (kind == 0x1009 || kind == 0x100c || kind == 0x100d) {
        const UCalendar *calendar = udat_getCalendar(format);
        int value = -1;
        if (kind == 0x100c) value = (ucal_getAttribute(calendar, UCAL_FIRST_DAY_OF_WEEK) + 5) % 7;
        else if (kind == 0x100d) {
            int days = ucal_getAttribute(calendar, UCAL_MINIMAL_DAYS_IN_FIRST_WEEK);
            value = days == 1 ? 0 : days == 7 ? 1 : days == 4 ? 2 : -1;
        } else {
            const char *name = ucal_getType(calendar, &error);
            if (name && U_SUCCESS(error)) value = !strcmp(name, "gregorian") ? 1 :
                !strcmp(name, "japanese") ? 3 : !strcmp(name, "buddhist") ? 7 :
                !strcmp(name, "hebrew") ? 8 : !strcmp(name, "persian") ? 22 :
                !strcmp(name, "islamic-umalqura") ? 23 : !strcmp(name, "islamic") ? 6 : -1;
        }
        char decimal[16]; int n = snprintf(decimal, sizeof(decimal), "%d", value);
        if (value >= 0 && n > 0 && n < capacity) { for (int i = 0; i < n; ++i) out[i] = (UChar)decimal[i]; result = n; }
        goto done;
    }
    if (kind >= 0x2a && kind <= 0x30) { type = UDAT_WEEKDAYS; index = (int32_t)((kind - 0x2a + 1) % 7 + 1); }
    else if (kind >= 0x31 && kind <= 0x37) { type = UDAT_SHORT_WEEKDAYS; index = (int32_t)((kind - 0x31 + 1) % 7 + 1); }
    else if (kind >= 0x38 && kind <= 0x43) { type = UDAT_MONTHS; index = (int32_t)(kind - 0x38); }
    else if (kind >= 0x44 && kind <= 0x4f) { type = UDAT_SHORT_MONTHS; index = (int32_t)(kind - 0x44); }
    else if (kind == 0x100e || kind == 0x100f) { type = kind == 0x100e ? UDAT_MONTHS : UDAT_SHORT_MONTHS; index = 12; }
    else if (kind == 0x28 || kind == 0x29) { type = UDAT_AM_PMS; index = (int32_t)(kind - 0x28); }
    else goto done;
    if (index >= udat_countSymbols(format, type)) { result = 0; goto done; }
    result = udat_getSymbols(format, type, index, out, capacity, &error);
    if (U_FAILURE(error) || result < 0 || result >= capacity) result = -1;
 done:
    udat_close(format);
    return result;
}
