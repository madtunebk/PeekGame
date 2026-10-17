/* Translate ICU currency field positions to Windows NLS pattern metadata. */
#include <unicode/unum.h>
#include <unicode/ufieldpositer.h>
#include <unicode/uchar.h>
#include <stdint.h>
#include <string.h>
int32_t peekgame_nls_currency_pattern(const UNumberFormat *format, uint32_t kind) {
    UErrorCode error = U_ZERO_ERROR;
    UFieldPositionIterator *fields = ufieldpositer_open(&error);
    if (U_FAILURE(error)) return -1;
    UChar text[256]; unsigned char roles[256] = {0}; char pattern[256];
    int negative = kind == 0x1c || kind == 0x53 || kind == 0x56 || kind == 0x57;
    UNumberFormat *copy = NULL;
    if (kind == 0x52) {
        copy = unum_clone(format, &error);
        if (U_FAILURE(error) || !copy) { ufieldpositer_close(fields); return -1; }
        unum_setAttribute(copy, UNUM_SIGN_ALWAYS_SHOWN, 1);
    }
    int32_t length = unum_formatDoubleForFields(copy ? copy : format, negative ? -1.0 : 1.0, text, 256, fields, &error);
    if (copy) unum_close(copy);
    if (U_FAILURE(error) || length < 0 || length >= 256) { ufieldpositer_close(fields); return -1; }
    int32_t start, end, field;
    while ((field = ufieldpositer_next(fields, &start, &end)) >= 0) {
        if (start < 0 || end > length) continue;
        int role = field == UNUM_CURRENCY_FIELD ? 1 : field == UNUM_SIGN_FIELD ? 3 :
            field == UNUM_INTEGER_FIELD || field == UNUM_FRACTION_FIELD || field == UNUM_DECIMAL_SEPARATOR_FIELD || field == UNUM_GROUPING_SEPARATOR_FIELD ? 2 : 0;
        if (role) for (int i = start; i < end; ++i) roles[i] = (unsigned char)role;
    }
    ufieldpositer_close(fields);
    int count = 0, previous = 0;
    for (int i = 0; i < length; ++i) {
        if (u_charType(text[i]) == U_FORMAT_CHAR) continue;
        int role = roles[i];
        if (role && role == previous) continue;
        char ch = role == 1 ? '$' : role == 2 ? 'n' : role == 3 ? '-' :
            u_isUWhiteSpace(text[i]) ? ' ' : text[i] == '(' || text[i] == ')' ? (char)text[i] : 0;
        if (!ch) return -1;
        pattern[count++] = ch; previous = role;
    }
    pattern[count] = 0;
    char *currency = strchr(pattern, '$'), *number = strchr(pattern, 'n');
    if (!currency || !number) return -1;
    if (kind == 0x54 || kind == 0x56) return currency < number;
    if (kind == 0x55 || kind == 0x57) {
        char *left = currency < number ? currency : number;
        char *right = currency < number ? number : currency;
        for (char *p = left; p < right; ++p) if (*p == ' ') return 1;
        return 0;
    }
    if (kind == 0x52 || kind == 0x53) {
        char *sign = strchr(pattern, '-');
        if (strchr(pattern, '(')) return 0;
        if (!sign) return -1;
        if (sign == pattern) return 1;
        if (!sign[1]) return 2;
        if (sign + 1 == currency) return 3;
        if (currency + 1 == sign) return 4;
        return -1;
    }
    const char *positive[] = {"$n", "n$", "$ n", "n $"};
    const char *negative_patterns[] = {"($n)", "-$n", "$-n", "$n-", "(n$)", "-n$", "n-$", "n$-", "-n $", "-$ n", "n $-", "$ n-", "$ -n", "n- $", "($ n)", "(n $)"};
    const char **patterns = negative ? negative_patterns : positive;
    int size = negative ? 16 : 4;
    for (int i = 0; i < size; ++i) if (!strcmp(pattern, patterns[i])) return i;
    return -1;
}
