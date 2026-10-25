#include "hopes.h"

void say(int fd, const char *msg)
{
    if (write(fd, msg, strlen(msg)) < 0)
        return;
}

void *xmalloc(size_t n)
{
    void *p = malloc(n ? n : 1);
    if (!p) {
        say(2, "hopes: out of memory\n");
        abort();
    }
    return p;
}

void *xrealloc(void *p, size_t n)
{
    p = realloc(p, n ? n : 1);
    if (!p) {
        say(2, "hopes: out of memory\n");
        abort();
    }
    return p;
}

char *xstrndup(const char *s, size_t n)
{
    char *r = xmalloc(n + 1);
    memcpy(r, s, n);
    r[n] = 0;
    return r;
}

static uint32_t unit16(const unsigned char *p, int big)
{
    return big ? (uint32_t)(p[0] << 8 | p[1]) : (uint32_t)(p[1] << 8 | p[0]);
}

static void append_utf8(String_Builder *out, uint32_t cp)
{
    char tmp[4];

    sb_append(out, tmp, (size_t)u8_enc(cp, tmp));
}

static void utf16_to_utf8(String_Builder *b, int big)
{
    String_Builder out = {0};
    const unsigned char *p = (const unsigned char *)b->data;
    size_t n = b->count & ~(size_t)1;

    sb_reserve(&out, n);
    for (size_t i = 2; i < n; i += 2) {
        uint32_t u = unit16(p + i, big), low;

        if (u >= 0xD800 && u < 0xDC00 && i + 3 < n && (low = unit16(p + i + 2, big)) >= 0xDC00 && low < 0xE000) {
            append_utf8(&out, 0x10000 + ((u - 0xD800) << 10) + (low - 0xDC00));
            i += 2;
        } else {
            append_utf8(&out, u >= 0xD800 && u < 0xE000 ? 0xFFFD : u);
        }
    }
    sb_free(b);
    *b = out;
}

void text_normalize(String_Builder *b)
{
    const unsigned char *p = (const unsigned char *)b->data;

    if (b->count >= 3 && p[0] == 0xEF && p[1] == 0xBB && p[2] == 0xBF)
        sb_remove(b, 0, 3);
    else if (b->count >= 2 && p[0] == 0xFF && p[1] == 0xFE)
        utf16_to_utf8(b, 0);
    else if (b->count >= 2 && p[0] == 0xFE && p[1] == 0xFF)
        utf16_to_utf8(b, 1);
}

void heap_trim(void)
{
#ifdef __APPLE__
    malloc_zone_pressure_relief(NULL, 0);
#elif defined(__GLIBC__)
    malloc_trim(0);
#endif
}

char *xstrdup(const char *s) { return xstrndup(s, strlen(s)); }

void snippet(char *out, size_t cap, const char *title, int max)
{
    size_t len = strlen(title), i = 0;
    int n = 0;
    while (i < len && n < max) {
        i = u8_next(title, len, i);
        n++;
    }
    int cut = i < len;
    if (i > cap - 4)
        i = cap - 4;
    memcpy(out, title, i);
    if (cut) {
        memcpy(out + i, "...", 3);
        i += 3;
    }
    out[i] = 0;
}

void tags_free(char **tags)
{
    for (int i = 0; i < len_of(tags); i++)
        free(tags[i]);
    arr_free(tags);
}

void item_free(Item *it)
{
    free(it->title);
    free(it->desc);
    tags_free(it->tags);
    memset(it, 0, sizeof *it);
}

void items_free(Item *v)
{
    for (int i = 0; i < len_of(v); i++)
        item_free(&v[i]);
    arr_free(v);
}

static const char *const PRIO_LABEL[] = {"Urgent", "High", "Medium", "Low"};
static const char *const PRIO_SHORT[] = {"Urg", "High", "Med", "Low"};
static const char *const PRIO_GLYPH[] = {"▲", "▲", "◆", "▽"};
static const char *const STATUS_LABEL[] = {"To Do", "In Progress", "Done"};
static const char *const STATUS_BADGE[] = {"○", "◐", "✓"};

const char *prio_label(int p) { return PRIO_LABEL[p]; }
const char *prio_short(int p) { return PRIO_SHORT[p]; }
const char *prio_glyph(int p) { return PRIO_GLYPH[p]; }
int prio_next(int p) { return (p + 1) & 3; }
int prio_prev(int p) { return (p + 3) & 3; }
const char *status_label(int s) { return STATUS_LABEL[s]; }
const char *status_badge(int s) { return STATUS_BADGE[s]; }
