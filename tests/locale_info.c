#include <windows.h>
#define CHECK(x) do { if (!(x)) ExitProcess(__LINE__); } while (0)
void entry(void) {
 WCHAR name[80], text[16]; DWORD cp = 0;
 CHECK(GetUserDefaultLocaleName(name, 80));
 CHECK(GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_IDEFAULTANSICODEPAGE | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 2) == 2);
 DWORD expected = name[0] == 'r' ? 1250 : name[0] == 'j' ? 932 : 1252;
 CHECK(cp == expected);
 int n = GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_IDEFAULTANSICODEPAGE, NULL, 0); CHECK(n > 1 && n < 16);
 CHECK(GetLocaleInfoEx(name, LOCALE_IDEFAULTANSICODEPAGE, text, 16) == n);
 DWORD parsed = 0; for (int i = 0; i < n - 1; ++i) { CHECK(text[i] >= '0' && text[i] <= '9'); parsed = parsed * 10 + text[i] - '0'; }
 CHECK(text[n - 1] == 0 && parsed == expected);
 CHECK(GetLocaleInfoEx(name, LOCALE_IDEFAULTANSICODEPAGE | LOCALE_RETURN_NUMBER, NULL, 0) == 2);
 cp = 99; CHECK(!GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_IDEFAULTANSICODEPAGE | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 1)); CHECK(GetLastError() == ERROR_INSUFFICIENT_BUFFER && cp == 99);
 CHECK(!GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_IDEFAULTANSICODEPAGE, text, 1)); CHECK(GetLastError() == ERROR_INSUFFICIENT_BUFFER);
 CHECK(!GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_IDEFAULTANSICODEPAGE, NULL, 6)); CHECK(GetLastError() == ERROR_INVALID_PARAMETER);
 CHECK(!GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_IDEFAULTANSICODEPAGE, text, -1)); CHECK(GetLastError() == ERROR_INVALID_PARAMETER);
 CHECK(GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_SENGLANGUAGE, text, 16) > 1);
 const WCHAR *language = expected == 1250 ? L"Romanian" : expected == 932 ? L"Japanese" : L"English";
 for (int i = 0;; ++i) { CHECK(text[i] == language[i]); if (!language[i]) break; }
 CHECK(GetLocaleInfoEx(name, LOCALE_SENGCOUNTRY, text, 16) > 1);
 const WCHAR *country = expected == 1250 ? L"Romania" : expected == 932 ? L"Japan" : L"United States";
 for (int i = 0;; ++i) { CHECK(text[i] == country[i]); if (!country[i]) break; }
 CHECK(!GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_SENGLANGUAGE | LOCALE_RETURN_NUMBER, text, 16)); CHECK(GetLastError() == ERROR_INVALID_FLAGS);
 CHECK(GetLocaleInfoEx(name, LOCALE_SINTLSYMBOL, text, 16) == 4);
 const WCHAR *currency = expected == 1250 ? L"RON" : expected == 932 ? L"JPY" : L"USD";
 for (int i = 0; i < 4; ++i) CHECK(text[i] == currency[i]);
 cp = 99; CHECK(GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_ICURRDIGITS | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 2) == 2); CHECK(cp == (expected == 932 ? 0 : 2));
 CHECK(GetLocaleInfoW(LOCALE_USER_DEFAULT, LOCALE_SCURRENCY, text, 16) > 1);
 WCHAR large[512]; CHECK(GetLocaleInfoEx(name, LOCALE_SENGCURRNAME, large, 512) > 1); CHECK(GetLocaleInfoEx(name, LOCALE_SNATIVECURRNAME, large, 512) > 1);
 CHECK(GetLocaleInfoEx(name, LOCALE_SMONGROUPING, text, 16) == 4); CHECK(text[0] == '3' && text[1] == ';' && text[2] == '0' && text[3] == 0);
 CHECK(GetLocaleInfoEx(name, LOCALE_SDECIMAL, text, 16) == 2); CHECK(text[0] == (expected == 1250 ? ',' : '.'));
 CHECK(GetLocaleInfoEx(name, LOCALE_SPOSITIVESIGN, text, 16) == 2 && text[0] == L'+');
 CHECK(GetLocaleInfoEx(name, LOCALE_SNEGATIVESIGN, text, 16) == 2 && text[0] == L'-');
 cp = 99; CHECK(GetLocaleInfoEx(name, LOCALE_IPOSSYMPRECEDES | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 2) == 2); CHECK(cp == (expected == 1250 ? 0 : 1));
 cp = 99; CHECK(GetLocaleInfoEx(name, LOCALE_IPOSSEPBYSPACE | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 2) == 2); CHECK(cp == (expected == 1250 ? 1 : 0));
 cp = 99; CHECK(GetLocaleInfoEx(name, LOCALE_ICURRENCY | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 2) == 2); CHECK(cp == (expected == 1250 ? 3 : 0));
 cp = 99; CHECK(GetLocaleInfoEx(name, LOCALE_INEGCURR | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 2) == 2); CHECK(cp < 16);
 CHECK(GetLocaleInfoEx(name, LOCALE_SDAYNAME1, text, 16) > 1);
 if (expected == 1252) { const WCHAR *monday = L"Monday"; for (int i = 0; i < 7; ++i) CHECK(text[i] == monday[i]); }
 CHECK(GetLocaleInfoEx(name, LOCALE_SABBREVDAYNAME7, text, 16) > 1);
 if (expected == 1252) CHECK(text[0] == 'S' && text[1] == 'u' && text[2] == 'n' && text[3] == 0);
 CHECK(GetLocaleInfoEx(name, LOCALE_SMONTHNAME1, text, 16) > 1);
 if (expected == 1252) { const WCHAR *january = L"January"; for (int i = 0; i < 8; ++i) CHECK(text[i] == january[i]); }
 CHECK(GetLocaleInfoEx(name, LOCALE_SABBREVMONTHNAME12, text, 16) > 1);
 CHECK(GetLocaleInfoEx(name, LOCALE_SMONTHNAME13, text, 16) == 1 && text[0] == 0);
 CHECK(GetLocaleInfoEx(name, LOCALE_S1159, text, 16) > 0);
 WCHAR pattern[256];
 CHECK(GetLocaleInfoEx(name, LOCALE_SSHORTDATE, pattern, 256) > 1);
 if (expected == 1252) { CHECK(pattern[0] == 'M'); }
 CHECK(GetLocaleInfoEx(name, LOCALE_SLONGDATE, pattern, 256) > 1);
 CHECK(GetLocaleInfoEx(name, LOCALE_STIMEFORMAT, pattern, 256) > 1);
 if (expected == 1252) { int tt = 0; for (int i = 0; pattern[i]; ++i) if (pattern[i] == 't' && pattern[i+1] == 't') tt = 1; CHECK(tt); }
 CHECK(GetLocaleInfoEx(name, LOCALE_SYEARMONTH, pattern, 256) > 1);
 cp = 0; CHECK(GetLocaleInfoEx(name, LOCALE_ICALENDARTYPE | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 2) == 2 && cp == CAL_GREGORIAN);
 cp = 99; CHECK(GetLocaleInfoEx(name, LOCALE_IFIRSTDAYOFWEEK | LOCALE_RETURN_NUMBER, (WCHAR *)&cp, 2) == 2); CHECK(cp == (expected == 1250 ? 0 : 6));
 CHECK(GetACP() == CP_UTF8); ExitProcess(0);
}
