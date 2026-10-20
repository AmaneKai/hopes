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
    if (j->p >= j->end) {
        sb_free(&b);
        return -1;
    }
    j->p++;
    *out = b.data;
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
    const char *start;
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
    start = j->p;
    while (j->p < j->end && strchr("+-.eE0123456789abcdefghijklmnopqrstuvwxyz", *j->p))
        j->p++;
    return j->p == start ? -1 : 0;
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

#define HAVE_TITLE 1
#define HAVE_PRIO 2
#define HAVE_STATUS 4

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

static int parse_field(Json_Parser *j, const char *key, Item *it, int *have)
{
    int v;

    if (!strcmp(key, "title")) {
        *have |= HAVE_TITLE;
        return parse_text(j, &it->title);
    }
    if (!strcmp(key, "description"))
        return parse_text(j, &it->desc);
    if (!strcmp(key, "tags"))
        return parse_tags(j, it);
    if (!strcmp(key, "priority")) {
        v = parse_enum(j, JSON_PRIOS, 4);
        it->prio = (uint8_t)v;
        *have |= HAVE_PRIO;
        return v < 0 ? -1 : 0;
    }
    if (!strcmp(key, "status")) {
        v = parse_enum(j, JSON_STATUSES, 3);
        it->status = (uint8_t)v;
        *have |= HAVE_STATUS;
        return v < 0 ? -1 : 0;
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
    if (more < 0 || have != (HAVE_TITLE | HAVE_PRIO | HAVE_STATUS))
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
    *out = v;
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
    bool pretty;
} Json_Writer;

static void indent(Json_Writer *w, int level)
{
    if (!w->pretty)
        return;
    sb_putc(w->out, '\n');
    for (int i = 0; i < level; i++)
        sb_append(w->out, "  ", 2);
}

static void member(Json_Writer *w, const char *name, bool first)
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

static void dump_item(Json_Writer *w, const Item *it)
{
    sb_putc(w->out, '{');
    member(w, "title", true);
    put_str(w->out, it->title);
    member(w, "description", false);
    put_str(w->out, it->desc);
    member(w, "priority", false);
    put_str(w->out, JSON_PRIOS[it->prio]);
    member(w, "status", false);
    put_str(w->out, JSON_STATUSES[it->status]);
    member(w, "tags", false);
    dump_tags(w, it->tags);
    indent(w, 1);
    sb_putc(w->out, '}');
}

void json_dump(String_Builder *o, const Item *v, int n, bool pretty)
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
