#include "hopes.h"

#define CE_UNIX_EPOCH 719163
#define MAX_DAY 3652059
#define MAX_INTERVAL 999
#define MAX_OFFSET 999999999
#define MON_FRI 0x1F
#define SAT_SUN 0x60
#define SOON_DAYS 2
#define WS " \t\n\r\v\f"
#define SEPARATORS " \t\n\r\v\f,"
#define EN_DASH "\xe2\x80\x93"

static const char *const WEEKDAYS[] = {"Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"};
static const char *const WEEKDAY_WORDS[7][4] = {
    {"mon", "monday"},
    {"tue", "tues", "tuesday"},
    {"wed", "weds", "wednesday"},
    {"thu", "thur", "thurs", "thursday"},
    {"fri", "friday"},
    {"sat", "saturday"},
    {"sun", "sunday"},
};

static int days_in_month(int y, int m)
{
    static const int len[] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    int leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;

    return len[m - 1] + (m == 2 && leap);
}

static int civil_valid(int64_t y, int64_t m, int64_t d)
{
    if (y < 1 || y > 9999 || m < 1 || m > 12)
        return 0;
    return d >= 1 && d <= days_in_month((int)y, (int)m);
}

Day day_make(int64_t y, int64_t m, int64_t d)
{
    int64_t era, yoe, doy, doe;

    if (!civil_valid(y, m, d))
        return NO_DAY;
    y -= m <= 2;
    era = y / 400;
    yoe = y - era * 400;
    doy = (153 * (m + (m > 2 ? -3 : 9)) + 2) / 5 + d - 1;
    doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return (Day)(era * 146097 + doe - 719468 + CE_UNIX_EPOCH);
}

void day_civil(Day day, int *y, int *m, int *d)
{
    int64_t z = (int64_t)day + 719468 - CE_UNIX_EPOCH, era = z / 146097, doe = z - era * 146097;
    int64_t yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    int64_t doy = doe - (365 * yoe + yoe / 4 - yoe / 100), mp = (5 * doy + 2) / 153;

    *d = (int)(doy - (153 * mp + 2) / 5 + 1);
    *m = (int)(mp < 10 ? mp + 3 : mp - 9);
    *y = (int)(yoe + era * 400 + (*m <= 2));
}

int day_weekday(Day day)
{
    return (day - 1) % 7;
}

Day day_today(void)
{
    struct tm tm;

    plat_localtime(time(NULL), &tm);
    return day_make(tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday);
}

Day day_add(Day day, int64_t n)
{
    int64_t r = (int64_t)day + n;

    return day && r >= 1 && r <= MAX_DAY ? (Day)r : NO_DAY;
}

static Day add_months(Day day, int64_t n)
{
    int y, m, d, len;
    int64_t total;

    if (!day)
        return NO_DAY;
    day_civil(day, &y, &m, &d);
    total = (int64_t)y * 12 + (m - 1) + n;
    if (total < 12 || total >= 10000 * 12)
        return NO_DAY;
    y = (int)(total / 12);
    m = (int)(total % 12) + 1;
    len = days_in_month(y, m);
    return day_make(y, m, d < len ? d : len);
}

static int weekday_of(const char *w)
{
    for (int i = 0; i < 7; i++)
        for (int k = 0; k < 4 && WEEKDAY_WORDS[i][k]; k++)
            if (!strcmp(w, WEEKDAY_WORDS[i][k]))
                return i;
    return -1;
}

static int is_one_of(const char *t, const char *a, const char *b, const char *c, const char *d)
{
    return !strcmp(t, a) || (b && !strcmp(t, b)) || (c && !strcmp(t, c)) || (d && !strcmp(t, d));
}

static char *trim_copy(const char *s, size_t n)
{
    size_t i = 0;

    while (i < n && isspace((unsigned char)s[i]))
        i++;
    while (n > i && isspace((unsigned char)s[n - 1]))
        n--;
    return xstrndup(s + i, n - i);
}

static char *normalize(const char *s)
{
    char *out = trim_copy(s, strlen(s));

    for (char *p = out; *p; p++)
        *p = (char)tolower((unsigned char)*p);
    return out;
}

static int parse_uint(const char *s, size_t n, int64_t *out)
{
    int64_t v = 0;

    if (!n || n > 9)
        return -1;
    for (size_t i = 0; i < n; i++) {
        if (!isdigit((unsigned char)s[i]))
            return -1;
        v = v * 10 + (s[i] - '0');
    }
    *out = v;
    return 0;
}

static const char *read_digits(const char *p, int min, int max, int *out)
{
    int n = 0, v = 0;

    while (n < max && isdigit((unsigned char)p[n]))
        v = v * 10 + (p[n++] - '0');
    if (n < min)
        return NULL;
    *out = v;
    return p + n;
}

static Day parse_iso(const char *t)
{
    int y, m, d;
    const char *p = read_digits(t, 4, 4, &y);

    if (!p || *p++ != '-')
        return NO_DAY;
    p = read_digits(p, 1, 2, &m);
    if (!p || *p++ != '-')
        return NO_DAY;
    p = read_digits(p, 1, 2, &d);
    return p && !*p ? day_make(y, m, d) : NO_DAY;
}

Day day_parse_iso(const char *s)
{
    return strlen(s) == 10 ? parse_iso(s) : NO_DAY;
}

void day_format_iso(Day day, char out[11])
{
    int y, m, d;

    day_civil(day, &y, &m, &d);
    snprintf(out, 11, "%04d-%02d-%02d", y, m, d);
}

static Day resolve_month_day(int64_t month, int64_t day, int64_t year, Day today)
{
    int ty, tm, td;
    Day this_year;

    if (year >= 0)
        return day_make(year, month, day);
    day_civil(today, &ty, &tm, &td);
    this_year = day_make(ty, month, day);
    if (!this_year)
        return NO_DAY;
    return this_year >= today ? this_year : day_make(ty + 1, month, day);
}

static int parse_part(const char *a, const char *b, int64_t *v, int *len)
{
    while (a < b && isspace((unsigned char)*a))
        a++;
    while (b > a && isspace((unsigned char)b[-1]))
        b--;
    *len = (int)(b - a);
    return parse_uint(a, (size_t)(b - a), v);
}

static Day parse_date(const char *t, Day today)
{
    int64_t parts[3];
    int lens[3], n = 0;
    const char *p = t;

    for (;;) {
        size_t len = strcspn(p, "/-");

        if (n == 3 || parse_part(p, p + len, &parts[n], &lens[n]))
            return NO_DAY;
        n++;
        if (!p[len])
            break;
        p += len + 1;
    }
    if (n == 2)
        return resolve_month_day(parts[0], parts[1], -1, today);
    if (n != 3 || (lens[2] != 2 && lens[2] != 4))
        return NO_DAY;
    return resolve_month_day(parts[0], parts[1], lens[2] == 2 ? 2000 + parts[2] : parts[2], today);
}

static Day parse_slice(const char *s, size_t n, Day base)
{
    char *t = trim_copy(s, n);
    Day d = parse_date(t, base);

    free(t);
    return d;
}

static int is_clear_word(const char *t)
{
    return !*t || !strcmp(t, "none") || !strcmp(t, "clear") || !strcmp(t, "-");
}

static const char *find_separator(const char *t, size_t *len)
{
    static const char *const seps[] = {" to ", " - ", " " EN_DASH " ", ".."};

    for (size_t i = 0; i < sizeof seps / sizeof *seps; i++) {
        const char *at = strstr(t, seps[i]);

        if (!at)
            continue;
        *len = strlen(seps[i]);
        return at;
    }
    return NULL;
}

static int parse_range(const char *t, const char *sep, size_t seplen, Day today, Window *w)
{
    char *end_text = trim_copy(sep + seplen, strlen(sep + seplen));
    Day start = parse_slice(t, (size_t)(sep - t), today), end;

    end = start ? parse_date(end_text, start) : NO_DAY;
    free(end_text);
    if (!start || !end || end < start)
        return -1;
    w->start = start < end ? start : NO_DAY;
    w->end = end;
    return 1;
}

int window_parse(const char *input, Day today, Window *w)
{
    char *t = normalize(input);
    const char *sep;
    size_t seplen = 0;
    int rc;

    memset(w, 0, sizeof *w);
    sep = find_separator(t, &seplen);
    if (sep)
        rc = parse_range(t, sep, seplen, today, w);
    else if (is_clear_word(t))
        rc = 0;
    else
        rc = (w->end = parse_date(t, today)) ? 1 : -1;
    free(t);
    return rc;
}

int day_parse(const char *input, Day today, Day *out)
{
    char *t = normalize(input);
    int rc = 0;

    *out = NO_DAY;
    if (!is_clear_word(t))
        rc = (*out = parse_date(t, today)) ? 0 : -1;
    free(t);
    return rc;
}

Urgency day_urgency(Day due, Day today)
{
    int64_t diff = (int64_t)due - today;

    if (diff < 0)
        return U_OVERDUE;
    if (diff == 0)
        return U_TODAY;
    return diff <= SOON_DAYS ? U_SOON : U_LATER;
}

Urgency window_urgency(const Window *w, Day today)
{
    if (w->start && w->start > today)
        return U_UPCOMING;
    return day_urgency(w->end, today);
}

void day_format_us(Day day, char out[11])
{
    int y, m, d;

    day_civil(day, &y, &m, &d);
    if (y >= 2000 && y <= 2099)
        snprintf(out, 11, "%02d-%02d-%02d", m, d, y % 100);
    else
        snprintf(out, 11, "%02d-%02d-%04d", m, d, y);
}

void day_relative_label(Day due, Day today, char *out, size_t cap)
{
    int days = due - today;

    if (days <= -2)
        snprintf(out, cap, "%dd late", -days);
    else if (days == -1)
        snprintf(out, cap, "yesterday");
    else if (days == 0)
        snprintf(out, cap, "today");
    else if (days == 1)
        snprintf(out, cap, "tomorrow");
    else if (days <= 6)
        snprintf(out, cap, "%s", WEEKDAYS[day_weekday(due)]);
    else
        day_format_us(due, out);
}

void window_label(const Window *w, Day today, char *out, size_t cap)
{
    char from[32];

    if (w->start && w->start > today) {
        day_relative_label(w->start, today, from, sizeof from);
        snprintf(out, cap, "from %s", from);
    } else if (w->start && w->end > today) {
        snprintf(out, cap, "%dd left", w->end - today);
    } else {
        day_relative_label(w->end, today, out, cap);
    }
}

void window_full_label(const Window *w, char *out, size_t cap)
{
    char start[11], end[11];

    day_format_us(w->end, end);
    if (!w->start) {
        snprintf(out, cap, "%s", end);
        return;
    }
    day_format_us(w->start, start);
    snprintf(out, cap, "%s " EN_DASH " %s", start, end);
}

Window window_moved_to(const Window *w, Day end)
{
    Window moved = {NO_DAY, end};

    if (w->start)
        moved.start = day_add(end, -((int64_t)w->end - w->start));
    return moved;
}

int item_window(const Item *it, Window *w)
{
    if (!it->due)
        return 0;
    w->start = it->start;
    w->end = it->due;
    return 1;
}

int item_overdue(const Item *it, Day today)
{
    return it->status != ST_DONE && it->due && it->due < today;
}

static Day next_weekday(unsigned mask, Day from)
{
    for (int offset = 0; offset < 7; offset++) {
        Day d = day_add(from, offset);

        if (!d)
            break;
        if (mask & (1u << day_weekday(d)))
            return d;
    }
    return from;
}

static int repeat_interval(const char *body, Repeat *r)
{
    size_t digits = strspn(body, "0123456789");
    const char *unit = body + digits;
    int64_t n;

    if (!*unit || parse_uint(body, digits, &n) || n == 0 || n > MAX_INTERVAL)
        return -1;
    unit += strspn(unit, WS);
    r->kind = R_EVERY;
    r->n = (uint16_t)n;
    if (is_one_of(unit, "d", "day", "days", NULL))
        r->unit = RU_DAY;
    else if (is_one_of(unit, "w", "wk", "week", "weeks"))
        r->unit = RU_WEEK;
    else if (is_one_of(unit, "m", "mo", "month", "months"))
        r->unit = RU_MONTH;
    else if (is_one_of(unit, "y", "yr", "year", "years"))
        r->unit = RU_YEAR;
    else
        return -1;
    return 0;
}

static int repeat_days(const char *body, Repeat *r)
{
    const char *p = body;
    unsigned mask = 0;

    for (;;) {
        char word[16];
        size_t len;
        int weekday;

        p += strspn(p, SEPARATORS);
        len = strcspn(p, SEPARATORS);
        if (!len)
            break;
        if (len >= sizeof word)
            return -1;
        memcpy(word, p, len);
        word[len] = 0;
        weekday = weekday_of(word);
        if (weekday < 0)
            return -1;
        mask |= 1u << weekday;
        p += len;
    }
    if (!mask)
        return -1;
    r->kind = R_ON;
    r->mask = (uint8_t)mask;
    return 0;
}

static int repeat_named(const char *body, Repeat *r)
{
    static const struct {
        const char *word;
        uint8_t kind, unit, mask;
    } names[] = {
        {"day", R_EVERY, RU_DAY, 0},     {"daily", R_EVERY, RU_DAY, 0},
        {"week", R_EVERY, RU_WEEK, 0},   {"weekly", R_EVERY, RU_WEEK, 0},
        {"month", R_EVERY, RU_MONTH, 0}, {"monthly", R_EVERY, RU_MONTH, 0},
        {"year", R_EVERY, RU_YEAR, 0},   {"yearly", R_EVERY, RU_YEAR, 0},
        {"annually", R_EVERY, RU_YEAR, 0}, {"weekday", R_ON, 0, MON_FRI},
        {"weekdays", R_ON, 0, MON_FRI},  {"weekend", R_ON, 0, SAT_SUN},
        {"weekends", R_ON, 0, SAT_SUN},
    };

    for (size_t i = 0; i < sizeof names / sizeof *names; i++) {
        if (strcmp(body, names[i].word))
            continue;
        r->kind = names[i].kind;
        r->unit = names[i].unit;
        r->mask = names[i].mask;
        r->n = 1;
        return 0;
    }
    return -1;
}

static int is_off_word(const char *t)
{
    return is_one_of(t, "", "none", "never", "off") || !strcmp(t, "-");
}

int repeat_parse(const char *input, Repeat *r)
{
    char *t = normalize(input);
    const char *body = !strncmp(t, "every", 5) ? t + 5 : t;
    int rc = 0;

    memset(r, 0, sizeof *r);
    body += strspn(body, WS);
    if (!is_off_word(t) && repeat_named(body, r) && repeat_interval(body, r)) {
        memset(r, 0, sizeof *r);
        rc = repeat_days(body, r);
    }
    free(t);
    return rc;
}

static void format_days(const Repeat *r, char *out, size_t cap)
{
    size_t used = 0;
    int count = 0, first = 1;

    for (int i = 0; i < 7; i++)
        count += (r->mask >> i) & 1;
    if (count == 1)
        used = (size_t)snprintf(out, cap, "every ");
    for (int i = 0; i < 7 && used < cap; i++) {
        if (!((r->mask >> i) & 1))
            continue;
        used += (size_t)snprintf(out + used, cap - used, "%s%s", first ? "" : ", ", WEEKDAYS[i]);
        first = 0;
    }
}

void repeat_format(const Repeat *r, char *out, size_t cap)
{
    static const char *const plain[] = {"daily", "weekly", "monthly", "yearly"};
    static const char letters[] = "dwmy";

    out[0] = 0;
    if (r->kind == R_EVERY && r->n == 1)
        snprintf(out, cap, "%s", plain[r->unit]);
    else if (r->kind == R_EVERY)
        snprintf(out, cap, "every %d%c", r->n, letters[r->unit]);
    else if (r->kind == R_ON && r->mask == MON_FRI)
        snprintf(out, cap, "weekdays");
    else if (r->kind == R_ON && r->mask == SAT_SUN)
        snprintf(out, cap, "weekends");
    else if (r->kind == R_ON)
        format_days(r, out, cap);
}

Day repeat_first_from(const Repeat *r, Day from)
{
    return r->kind == R_ON ? next_weekday(r->mask, from) : from;
}

static Day step_days(Day base, int64_t step, Day earliest)
{
    int64_t gap = (int64_t)earliest - base, steps = (gap + step - 1) / step;

    return day_add(base, (steps > 1 ? steps : 1) * step);
}

static Day step_months(Day base, int64_t step, Day earliest)
{
    for (int64_t k = 1;; k++) {
        Day date = add_months(base, step * k);

        if (!date)
            return earliest;
        if (date >= earliest)
            return date;
    }
}

Day repeat_next_after(const Repeat *r, Day base, Day today)
{
    Day after = day_add(base, 1), earliest = after > today ? after : today;

    if (r->kind == R_ON)
        return next_weekday(r->mask, earliest);
    if (r->unit == RU_DAY)
        return step_days(base, r->n, earliest);
    if (r->unit == RU_WEEK)
        return step_days(base, r->n * 7, earliest);
    return step_months(base, r->unit == RU_YEAR ? r->n * 12 : r->n, earliest);
}
