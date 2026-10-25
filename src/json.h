#include "hopes.h"

typedef struct {
    const char *p, *end;
} Json_Parser;

static void ws(Json_Parser *j)
{
    while (j->p < j->end && *j->p && strchr(" \n\t\r", *j->p))
        j->p++;
}

static const char HEX_DIGITS[] = "0123456789abcdef";
static const char ESC_FROM[] = "\"\\/bfnrt";
static const char ESC_TO[] = "\"\\/\b\f\n\r\t";

static int utf8_valid(const unsigned char *s, size_t n)
{
    for (size_t i = 0; i < n;) {
        unsigned char c = s[i];
        size_t len = c < 0x80 ? 1 : c >= 0xC2 && c < 0xE0 ? 2 : c >= 0xE0 && c < 0xF0 ? 3 : c >= 0xF0 && c < 0xF5 ? 4 : 0;
        uint32_t cp = len == 1 ? c : c & (0xFFu >> (len + 1));

        if (!len || i + len > n)
            return 0;
        for (size_t k = 1; k < len; k++) {
            if ((s[i + k] & 0xC0) != 0x80)
                return 0;
            cp = cp << 6 | (s[i + k] & 0x3F);
        }
        if ((len == 3 && cp < 0x800) || (len == 4 && cp < 0x10000) || cp > 0x10FFFF)
            return 0;
        if (cp >= 0xD800 && cp < 0xE000)
            return 0;
        i += len;
    }
    return 1;
}

static int hex4(Json_Parser *j, uint32_t *out)
{
    uint32_t v = 0;

    if (j->end - j->p < 4)
        return -1;
    for (int i = 0; i < 4; i++) {
        const char *d = strchr(HEX_DIGITS, *j->p | 0x20);

        if (!d)
            return -1;
        v = v << 4 | (uint32_t)(d - HEX_DIGITS);
        j->p++;
    }
    *out = v;
    return 0;
}

static int parse_unicode(Json_Parser *j, uint32_t *cp)
{
    uint32_t lo;

    if (hex4(j, cp))
        return -1;
    if (*cp >= 0xDC00 && *cp < 0xE000)
        return -1;
    if (*cp < 0xD800 || *cp >= 0xDC00)
        return 0;
    if (j->end - j->p < 6 || j->p[0] != '\\' || j->p[1] != 'u')
        return -1;
    j->p += 2;
    if (hex4(j, &lo) || lo < 0xDC00 || lo > 0xDFFF)
        return -1;
    *cp = 0x10000 + ((*cp - 0xD800) << 10) + (lo - 0xDC00);
    return 0;
}

static int parse_escape(Json_Parser *j, String_Builder *b)
{
    char tmp[4], e;
    uint32_t cp;

    if (j->p >= j->end)
        return -1;
    e = *j->p++;
    if (e == 'u') {
        if (parse_unicode(j, &cp))
            return -1;
        if (!cp)
            cp = 0xFFFD;
        sb_append(b, tmp, (size_t)u8_enc(cp, tmp));
        return 0;
    }
    const char *at = e ? strchr(ESC_FROM, e) : NULL;

    if (!at)
        return -1;
    sb_putc(b, ESC_TO[at - ESC_FROM]);
    return 0;
}

static int parse_string(Json_Parser *j, char **out)
{
    String_Builder b = {0};

    if (j->p >= j->end || *j->p != '"')
        return -1;
    j->p++;
    sb_reserve(&b, 0);
    while (j->p < j->end && *j->p != '"') {
        unsigned char c = (unsigned char)*j->p++;

        if (c < 0x20 || (c == '\\' && parse_escape(j, &b))) {
            sb_free(&b);
            return -1;
        }
        if (c != '\\')
            sb_putc(&b, (char)c);
    }
    if (j->p >= j->end || !utf8_valid((const unsigned char *)b.data, b.count)) {
        sb_free(&b);
        return -1;
    }
    j->p++;
    *out = xrealloc(b.data, b.count + 1);
    return 0;
}

static int list_open(Json_Parser *j, char open, char close)
{
    ws(j);
    if (j->p >= j->end || *j->p++ != open)
        return -1;
    ws(j);
    if (j->p < j->end && *j->p == close) {
        j->p++;
        return 0;
    }
    return 1;
}

static int list_next(Json_Parser *j, char close)
{
    ws(j);
    if (j->p >= j->end)
        return -1;
    if (*j->p == ',') {
        j->p++;
        return 1;
    }
    if (*j->p != close)
        return -1;
    j->p++;
    return 0;
}

static int parse_key(Json_Parser *j, char **key)
{
    ws(j);
    if (parse_string(j, key))
        return -1;
    ws(j);
    if (j->p >= j->end || *j->p++ != ':') {
        free(*key);
        return -1;
    }
    ws(j);
    return 0;
}

static int skip_value(Json_Parser *j, int depth);

static int skip_word(Json_Parser *j, const char *word)
{
    size_t n = strlen(word);

    if ((size_t)(j->end - j->p) < n || strncmp(j->p, word, n))
        return -1;
    j->p += n;
    return 0;
}

static int skip_digits(Json_Parser *j)
{
    const char *start = j->p;

    while (j->p < j->end && isdigit((unsigned char)*j->p))
        j->p++;
    return j->p == start ? -1 : 0;
}

static int skip_number(Json_Parser *j)
{
    if (j->p < j->end && *j->p == '-')
        j->p++;
    if (j->p < j->end && *j->p == '0')
        j->p++;
    else if (skip_digits(j))
        return -1;
    if (j->p < j->end && *j->p == '.') {
        j->p++;
        if (skip_digits(j))
            return -1;
    }
    if (j->p < j->end && (*j->p == 'e' || *j->p == 'E')) {
        j->p++;
        if (j->p < j->end && (*j->p == '+' || *j->p == '-'))
            j->p++;
        if (skip_digits(j))
            return -1;
    }
    return 0;
}

static int skip_scalar(Json_Parser *j)
{
    if (*j->p == 't')
        return skip_word(j, "true");
    if (*j->p == 'f')
        return skip_word(j, "false");
    if (*j->p == 'n')
        return skip_word(j, "null");
    return skip_number(j);
}

static int skip_container(Json_Parser *j, int depth)
{
    char close = *j->p == '{' ? '}' : ']';
    int more = list_open(j, *j->p, close);

    while (more > 0) {
        char *key;

        if (close == '}' && !parse_key(j, &key))
            free(key);
        else if (close == '}')
            return -1;
        if (skip_value(j, depth + 1))
            return -1;
        more = list_next(j, close);
    }
    return more;
}

static int skip_value(Json_Parser *j, int depth)
{
    char *s;

    ws(j);
    if (depth > 64 || j->p >= j->end)
        return -1;
    if (*j->p == '{' || *j->p == '[')
        return skip_container(j, depth);
    if (*j->p == '"') {
        if (parse_string(j, &s))
            return -1;
        free(s);
        return 0;
    }
    return skip_scalar(j);
}

static void *arr_trim(void *arr, size_t item)
{
    size_t n = arr_len(arr);
    _Arr_Header *h;

    if (!n || arr_cap(arr) == n)
        return arr;
    h = xrealloc(_arr_header(arr), offsetof(_Arr_Header, data) + n * item);
    h->capacity = n;
    return h->data;
}

static int parse_tags(Json_Parser *j, Item *it)
{
    int more = list_open(j, '[', ']');

    while (more > 0) {
        char *s;

        ws(j);
        if (parse_string(j, &s))
            return -1;
        arr_push(it->tags, s);
        more = list_next(j, ']');
    }
    it->tags = arr_trim(it->tags, sizeof *it->tags);
    return more;
}

static int enum_of(const char *s, const char *const *names, int n)
{
    for (int i = 0; i < n; i++)
        if (!strcmp(s, names[i]))
            return i;
    return -1;
}

static const char *const JSON_PRIOS[] = {"Urgent", "High", "Medium", "Low"};
static const char *const JSON_STATUSES[] = {"Todo", "InProgress", "Complete"};

enum {
    F_TITLE = 1,
    F_DESC = 2,
    F_PRIO = 4,
    F_STATUS = 8,
    F_TAGS = 16,
    F_DUE = 32,
    F_START = 64,
    F_REPEAT = 128,
    F_REQUIRED = F_TITLE | F_PRIO | F_STATUS
};

static const struct {
    const char *name;
    int bit;
} FIELDS[] = {
    {"title", F_TITLE}, {"description", F_DESC}, {"priority", F_PRIO}, {"status", F_STATUS},
    {"tags", F_TAGS},   {"due", F_DUE},          {"start", F_START},   {"repeat", F_REPEAT},
};

static int parse_enum(Json_Parser *j, const char *const *names, int n)
{
    char *val;
    int v;

    if (parse_string(j, &val))
        return -1;
    v = enum_of(val, names, n);
    free(val);
    return v;
}

static int parse_text(Json_Parser *j, char **dst)
{
    char *val;

    if (parse_string(j, &val))
        return -1;
    free(*dst);
    *dst = val;
    return 0;
}

static int parse_nullable(Json_Parser *j, char **out)
{
    ws(j);
    *out = NULL;
    if (j->p < j->end && *j->p == 'n')
        return skip_word(j, "null");
    return parse_string(j, out);
}

static int parse_day_field(Json_Parser *j, Day *dst)
{
    char *s;
    int bad;

    if (parse_nullable(j, &s))
        return -1;
    if (s)
        *dst = day_parse_iso(s);
    bad = s && !*dst;
    free(s);
    return bad ? -1 : 0;
}

static int parse_repeat_field(Json_Parser *j, Repeat *dst)
{
    char *s;
    int bad;

    if (parse_nullable(j, &s))
        return -1;
    bad = s && (repeat_parse(s, dst) || dst->kind == R_NONE);
    free(s);
    return bad ? -1 : 0;
}

static int parse_value(Json_Parser *j, int bit, Item *it)
{
    int v;

    switch (bit) {
    case F_TITLE: return parse_text(j, &it->title);
    case F_DESC: return parse_text(j, &it->desc);
    case F_TAGS: return parse_tags(j, it);
    case F_DUE: return parse_day_field(j, &it->due);
    case F_START: return parse_day_field(j, &it->start);
    case F_REPEAT: return parse_repeat_field(j, &it->repeat);
    case F_PRIO:
        v = parse_enum(j, JSON_PRIOS, 4);
        it->prio = (uint8_t)v;
        return v < 0 ? -1 : 0;
    default:
        v = parse_enum(j, JSON_STATUSES, 3);
        it->status = (uint8_t)v;
        return v < 0 ? -1 : 0;
    }
}

static int parse_field(Json_Parser *j, const char *key, Item *it, int *have)
{
    for (size_t i = 0; i < sizeof FIELDS / sizeof *FIELDS; i++) {
        if (strcmp(key, FIELDS[i].name))
            continue;
        if (*have & FIELDS[i].bit)
            return -1;
        *have |= FIELDS[i].bit;
        return parse_value(j, FIELDS[i].bit, it);
    }
    return skip_value(j, 0);
}

static int parse_item(Json_Parser *j, Item *it)
{
    int have = 0, more = list_open(j, '{', '}');

    memset(it, 0, sizeof *it);
    while (more > 0) {
        char *key;
        int rc;

        if (parse_key(j, &key))
            return -1;
        rc = parse_field(j, key, it, &have);
        free(key);
        if (rc)
            return -1;
        more = list_next(j, '}');
    }
    if (more < 0 || (have & F_REQUIRED) != F_REQUIRED)
        return -1;
    if (!it->desc)
        it->desc = xstrdup("");
    return 0;
}

int json_load(const char *data, size_t len, Item **out)
{
    Json_Parser j = {data, data + len};
    Item *v = NULL, it;
    int more = list_open(&j, '[', ']');

    while (more > 0) {
        if (parse_item(&j, &it)) {
            item_free(&it);
            more = -1;
            break;
        }
        arr_push(v, it);
        more = list_next(&j, ']');
    }
    ws(&j);
    if (more < 0 || j.p != j.end) {
        items_free(v);
        return -1;
    }
    *out = arr_trim(v, sizeof *v);
    return 0;
}

static void put_str(String_Builder *o, const char *s)
{
    static const char named[] = "\"\\\n\r\t\b\f";
    static const char *const names[] = {"\\\"", "\\\\", "\\n", "\\r", "\\t", "\\b", "\\f"};

    sb_putc(o, '"');
    for (; *s; s++) {
        unsigned char c = (unsigned char)*s;
        const char *at = c ? strchr(named, c) : NULL;
        char e[6] = {'\\', 'u', '0', '0', HEX_DIGITS[c >> 4], HEX_DIGITS[c & 15]};

        if (at)
            sb_append(o, names[at - named], 2);
        else if (c < 0x20)
            sb_append(o, e, 6);
        else
            sb_putc(o, (char)c);
    }
    sb_putc(o, '"');
}

typedef struct {
    String_Builder *out;
    int pretty;
} Json_Writer;

static void indent(Json_Writer *w, int level)
{
    if (!w->pretty)
        return;
    sb_putc(w->out, '\n');
    for (int i = 0; i < level; i++)
        sb_append(w->out, "  ", 2);
}

static void member(Json_Writer *w, const char *name, int first)
{
    if (!first)
        sb_putc(w->out, ',');
    indent(w, 2);
    put_str(w->out, name);
    sb_append(w->out, w->pretty ? ": " : ":", w->pretty ? 2 : 1);
}

static void dump_tags(Json_Writer *w, char *const *tags)
{
    sb_putc(w->out, '[');
    for (int t = 0; t < len_of(tags); t++) {
        if (t)
            sb_putc(w->out, ',');
        indent(w, 3);
        put_str(w->out, tags[t]);
    }
    if (len_of(tags))
        indent(w, 2);
    sb_putc(w->out, ']');
}

static void dump_day(Json_Writer *w, const char *name, Day day)
{
    char iso[11];

    if (!day)
        return;
    day_format_iso(day, iso);
    member(w, name, 0);
    put_str(w->out, iso);
}

static void dump_repeat(Json_Writer *w, const Repeat *r)
{
    char rule[48];

    if (r->kind == R_NONE)
        return;
    repeat_format(r, rule, sizeof rule);
    member(w, "repeat", 0);
    put_str(w->out, rule);
}

static void dump_item(Json_Writer *w, const Item *it)
{
    sb_putc(w->out, '{');
    member(w, "title", 1);
    put_str(w->out, it->title);
    member(w, "description", 0);
    put_str(w->out, it->desc);
    member(w, "priority", 0);
    put_str(w->out, JSON_PRIOS[it->prio]);
    member(w, "status", 0);
    put_str(w->out, JSON_STATUSES[it->status]);
    member(w, "tags", 0);
    dump_tags(w, it->tags);
    dump_day(w, "due", it->due);
    dump_day(w, "start", it->start);
    dump_repeat(w, &it->repeat);
    indent(w, 1);
    sb_putc(w->out, '}');
}

void json_dump(String_Builder *o, const Item *v, int n, int pretty)
{
    Json_Writer w = {o, pretty};

    sb_reserve(o, 64 + (size_t)n * 128);
    sb_putc(o, '[');
    for (int i = 0; i < n; i++) {
        if (i)
            sb_putc(o, ',');
        indent(&w, 1);
        dump_item(&w, &v[i]);
    }
    if (n)
        indent(&w, 0);
    sb_putc(o, ']');
}
