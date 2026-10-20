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
