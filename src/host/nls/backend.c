/* Host ICU locale metadata; no ICU object crosses the Windows ABI. */
#include <unicode/unum.h>
#include <unicode/ucurr.h>
#include <unicode/ustring.h>
#include <stdio.h>
#include <stdint.h>
int32_t peekgame_nls_date_pattern(const char *, uint32_t, UChar *, int32_t);
int32_t peekgame_nls_calendar(const char *, uint32_t, UChar *, int32_t);
int32_t peekgame_nls_currency_pattern(const UNumberFormat *, uint32_t);
int32_t peekgame_nls_number(const char *locale, uint32_t kind, UChar *out, int32_t capacity) {
    if ((kind >= 0x1d && kind <= 0x27) || kind == 0x78 || kind == 0x79 || kind == 0x1003 || kind == 0x1005 || kind == 0x1006)
        return peekgame_nls_date_pattern(locale, kind, out, capacity);
    if ((kind >= 0x28 && kind <= 0x4f) || kind == 0x1009 || (kind >= 0x100c && kind <= 0x100f))
        return peekgame_nls_calendar(locale, kind, out, capacity);
    UErrorCode error = U_ZERO_ERROR;
    UNumberFormat *format = unum_open(UNUM_CURRENCY, NULL, 0, locale, NULL, &error);
    if (U_FAILURE(error) || !format) return -1;
    int32_t result = -1;
    UNumberFormatSymbol symbol;
    switch (kind) {
    case 0x0e: symbol = UNUM_DECIMAL_SEPARATOR_SYMBOL; break;
    case 0x0f: symbol = UNUM_GROUPING_SEPARATOR_SYMBOL; break;
    case 0x1b: case 0x1c: case 0x52: case 0x53: case 0x54: case 0x55: case 0x56: case 0x57: {
        int32_t value = peekgame_nls_currency_pattern(format, kind);
        char decimal[16]; int n = snprintf(decimal, sizeof(decimal), "%d", value);
        if (value >= 0 && n > 0 && n < capacity) { for (int i = 0; i < n; ++i) out[i] = (UChar)decimal[i]; result = n; }
        goto done;
    }
    case 0x50: symbol = UNUM_PLUS_SIGN_SYMBOL; break;
    case 0x51: symbol = UNUM_MINUS_SIGN_SYMBOL; break;
    case 0x14: symbol = UNUM_CURRENCY_SYMBOL; break;
    case 0x15: symbol = UNUM_INTL_CURRENCY_SYMBOL; break;
    case 0x16: symbol = UNUM_MONETARY_SEPARATOR_SYMBOL; break;
    case 0x17: symbol = UNUM_MONETARY_GROUPING_SEPARATOR_SYMBOL; break;
    case 0x1007: case 0x1008: {
        UChar currency[8];
        unum_getTextAttribute(format, UNUM_CURRENCY_CODE, currency, 8, &error);
        UBool choice = 0; int32_t length = 0;
        const UChar *name = U_FAILURE(error) ? NULL : ucurr_getName(currency,
            kind == 0x1007 ? "en" : locale, UCURR_LONG_NAME, &choice, &length, &error);
        if (name && U_SUCCESS(error) && length < capacity) { u_memcpy(out, name, length); result = length; }
        goto done;
    }
    case 0x10: case 0x18: {
        int32_t primary = unum_getAttribute(format, UNUM_GROUPING_SIZE);
        int32_t secondary = unum_getAttribute(format, UNUM_SECONDARY_GROUPING_SIZE);
        if (!unum_getAttribute(format, UNUM_GROUPING_USED) || primary == 0) {
            if (capacity > 0) { out[0] = '0'; result = 1; }
        } else {
            char grouping[32];
            int count = secondary > 0 && secondary != primary
                ? snprintf(grouping, sizeof(grouping), "%d;%d;0", primary, secondary)
                : snprintf(grouping, sizeof(grouping), "%d;0", primary);
            if (count > 0 && count < (int)sizeof(grouping) && count < capacity) {
                for (int i = 0; i < count; ++i) out[i] = (UChar)grouping[i];
                result = count;
            }
        }
        goto done;
    }
    case 0x19: case 0x1a: {
        UChar currency[8];
        unum_getTextAttribute(format, UNUM_CURRENCY_CODE, currency, 8, &error);
        int32_t digits = U_FAILURE(error) ? -1 : ucurr_getDefaultFractionDigits(currency, &error);
        if (U_SUCCESS(error) && digits >= 0 && digits < 10 && capacity >= 1) { out[0] = (UChar)('0' + digits); result = 1; }
        goto done;
    }
    default: goto done;
    }
    result = unum_getSymbol(format, symbol, out, capacity, &error);
    if (U_FAILURE(error) || result < 0 || result >= capacity) result = -1;
 done:
    unum_close(format);
    return result;
}
