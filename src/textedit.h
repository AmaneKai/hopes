#include "hopes.h"

#define S(b) ((b)->data)

static size_t nxt(const String_Builder *b, size_t i) { return u8_next(S(b), b->count, i); }
static size_t prv(const String_Builder *b, size_t i) { return u8_prev(S(b), i); }

static int is_word(const String_Builder *b, size_t i)
{
    unsigned char c = (unsigned char)S(b)[i];
    return !(c == ' ' || (c >= 9 && c <= 13));
}

static size_t last_char(const String_Builder *b) { return b->count ? prv(b, b->count) : 0; }

static size_t lstart(const String_Builder *b, size_t cur)
{
    while (cur > 0 && S(b)[cur - 1] != '\n')
        cur--;
    return cur;
}

static size_t lend(const String_Builder *b, size_t cur)
{
    while (cur < b->count && S(b)[cur] != '\n')
        cur++;
    return cur;
}

static size_t last_pos(const String_Builder *b, size_t st, size_t en) { return en > st ? prv(b, en) : st; }

static size_t min_z(size_t a, size_t b) { return a < b ? a : b; }

static size_t cols(const String_Builder *b, size_t from, size_t to) { return u8_chars(S(b) + from, to - from); }

static size_t adv(const String_Builder *b, size_t from, size_t n, size_t limit)
{
    while (n-- && from < limit)
        from = nxt(b, from);
    return from < limit ? from : limit;
}

size_t te_clamp_normal(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur), hi = last_pos(b, st, lend(b, cur));
    return cur < st ? st : cur > hi ? hi : cur;
}

size_t te_move_left(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur);
    return cur > st ? prv(b, cur) : st;
}

size_t te_move_right(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur);
    return min_z(nxt(b, cur), last_pos(b, st, lend(b, cur)));
}

size_t te_move_up(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur);
    if (st == 0)
        return cur;
    size_t col = cols(b, st, cur), pend = st - 1, pst = lstart(b, pend);
    return min_z(adv(b, pst, col, pend), last_pos(b, pst, pend));
}

size_t te_move_down(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur), en = lend(b, cur);
    if (en >= b->count)
        return cur;
    size_t col = cols(b, st, cur), nst = en + 1, nen = lend(b, nst);
    return min_z(adv(b, nst, col, nen), last_pos(b, nst, nen));
}

size_t te_caret_left(const String_Builder *b, size_t cur)
{
    cur = min_z(cur, b->count);
    size_t st = lstart(b, cur);
    return cur > st ? prv(b, cur) : st;
}

size_t te_caret_right(const String_Builder *b, size_t cur)
{
    cur = min_z(cur, b->count);
    return min_z(nxt(b, cur), lend(b, cur));
}

size_t te_caret_up(const String_Builder *b, size_t cur)
{
    cur = min_z(cur, b->count);
    size_t st = lstart(b, cur);
    if (st == 0)
        return cur;
    size_t col = cols(b, st, cur), pend = st - 1, pst = lstart(b, pend);
    return adv(b, pst, col, pend);
}

size_t te_caret_down(const String_Builder *b, size_t cur)
{
    cur = min_z(cur, b->count);
    size_t st = lstart(b, cur), en = lend(b, cur);
    if (en >= b->count)
        return cur;
    size_t col = cols(b, st, cur), nst = en + 1;
    return adv(b, nst, col, lend(b, nst));
}

size_t te_line_start(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    return lstart(b, min_z(cur, last_char(b)));
}

size_t te_line_end(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur);
    return last_pos(b, st, lend(b, cur));
}

size_t te_word_forward(const String_Builder *b, size_t cur)
{
    size_t n = b->count;
    if (!n)
        return 0;
    size_t i = min_z(cur, last_char(b));
    if (is_word(b, i))
        while (i < n && is_word(b, i))
            i = nxt(b, i);
    while (i < n && !is_word(b, i))
        i = nxt(b, i);
    return i >= n ? last_char(b) : i;
}

size_t te_word_end(const String_Builder *b, size_t cur)
{
    size_t n = b->count;
    if (!n)
        return 0;
    size_t i = nxt(b, min_z(cur, last_char(b)));
    while (i < n && !is_word(b, i))
        i = nxt(b, i);
    while (i < n) {
        size_t j = nxt(b, i);
        if (j < n && is_word(b, j))
            i = j;
        else
            break;
    }
    return min_z(i, last_char(b));
}

size_t te_word_back(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    size_t i = min_z(cur, last_char(b));
    if (i == 0)
        return 0;
    i = prv(b, i);
    while (i > 0 && !is_word(b, i))
        i = prv(b, i);
    while (i > 0 && is_word(b, prv(b, i)))
        i = prv(b, i);
    return i;
}

size_t te_insert(String_Builder *b, size_t cur, const char *s, size_t n)
{
    cur = min_z(cur, b->count);
    sb_insert(b, cur, s, n);
    return cur + n;
}

size_t te_backspace(String_Builder *b, size_t cur)
{
    cur = min_z(cur, b->count);
    if (cur == 0)
        return 0;
    size_t p = prv(b, cur);
    sb_remove(b, p, cur - p);
    return p;
}

size_t te_delete_char(String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur);
    if (lend(b, cur) > st)
        sb_remove(b, cur, nxt(b, cur) - cur);
    return te_clamp_normal(b, cur);
}

size_t te_delete_to_line_end(String_Builder *b, size_t cur)
{
    cur = min_z(cur, b->count);
    size_t st = lstart(b, cur), en = lend(b, cur);
    if (en > cur)
        sb_remove(b, cur, en - cur);
    if (cur < st)
        cur = st;
    return te_clamp_normal(b, min_z(cur, b->count));
}

size_t te_delete_line(String_Builder *b, size_t cur)
{
    size_t n = b->count;
    if (!n)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur), en = lend(b, cur), rs = st, re = en;
    if (en < n)
        re = en + 1;
    else if (st > 0)
        rs = st - 1;
    sb_remove(b, rs, re - rs);
    return te_clamp_normal(b, rs);
}

size_t te_insert_after(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    cur = min_z(cur, last_char(b));
    size_t st = lstart(b, cur), n = min_z(nxt(b, cur), lend(b, cur));
    return n < st ? st : n;
}

size_t te_insert_line_end(const String_Builder *b, size_t cur)
{
    if (!b->count)
        return 0;
    return lend(b, min_z(cur, last_char(b)));
}

size_t te_open_below(String_Builder *b, size_t cur, int allow_nl)
{
    if (!allow_nl)
        return te_insert_line_end(b, cur);
    size_t en = lend(b, b->count ? min_z(cur, last_char(b)) : 0);
    sb_insert(b, en, "\n", 1);
    return en + 1;
}

size_t te_open_above(String_Builder *b, size_t cur, int allow_nl)
{
    if (!allow_nl)
        return te_line_start(b, cur);
    size_t st = lstart(b, b->count ? min_z(cur, last_char(b)) : 0);
    sb_insert(b, st, "\n", 1);
    return st;
}

char *te_yank(const String_Builder *b, size_t lo, size_t hi)
{
    if (!b->count || lo >= b->count)
        return xstrdup("");
    hi = min_z(hi, last_char(b));
    return xstrndup(S(b) + lo, nxt(b, hi) - lo);
}

size_t te_delete_range(String_Builder *b, size_t lo, size_t hi)
{
    if (!b->count || lo >= b->count)
        return te_clamp_normal(b, lo);
    hi = min_z(hi, last_char(b));
    sb_remove(b, lo, nxt(b, hi) - lo);
    return te_clamp_normal(b, lo);
}

static size_t paste_at(String_Builder *b, size_t at, const char *s)
{
    size_t n = strlen(s);
    sb_insert(b, at, s, n);
    return te_clamp_normal(b, prv(b, at + n));
}

size_t te_paste_after(String_Builder *b, size_t cur, const char *s)
{
    if (!*s)
        return cur;
    size_t at = cur < b->count ? nxt(b, cur) : b->count;
    return paste_at(b, at, s);
}

size_t te_paste_before(String_Builder *b, size_t cur, const char *s)
{
    if (!*s)
        return cur;
    return paste_at(b, min_z(cur, b->count), s);
}

size_t te_delete_word_back(String_Builder *b, size_t cur)
{
    size_t i = min_z(cur, b->count);
    while (i > 0 && S(b)[i - 1] == ' ')
        i--;
    while (i > 0 && S(b)[i - 1] != ' ' && S(b)[i - 1] != '\n')
        i--;
    sb_remove(b, i, cur - i);
    return i;
}

void ti_init(Text_Input *t)
{
    memset(t, 0, sizeof *t);
    t->insert = 1;
    t->anchor = -1;
}

void ti_focus(Text_Input *t, const String_Builder *b)
{
    t->insert = 1;
    t->pending_d = 0;
    t->anchor = -1;
    t->cursor = b->count;
}

int ti_esc(Text_Input *t, String_Builder *b)
{
    if (t->insert) {
        t->insert = 0;
        t->pending_d = 0;
        t->cursor = te_clamp_normal(b, t->cursor);
        return 1;
    }
    if (t->anchor >= 0) {
        t->anchor = -1;
        return 1;
    }
    return 0;
}

static void ti_set_clip(Text_Input *t, char *s)
{
    free(t->clip);
    t->clip = s;
}

static void insert_key(Text_Input *t, String_Builder *b, Key k, int allow_nl)
{
    if (k.ctrl) {
        if (k.c == 'w')
            t->cursor = te_delete_word_back(b, t->cursor);
        else if (k.c == 'u') {
            sb_clear(b);
            t->cursor = 0;
        }
        return;
    }
    switch (k.c) {
    case HK_ENTER:
        if (allow_nl)
            t->cursor = te_insert(b, t->cursor, "\n", 1);
        return;
    case HK_BACKSPACE:
        t->cursor = te_backspace(b, t->cursor);
        return;
    case HK_LEFT:
        t->cursor = te_caret_left(b, t->cursor);
        return;
    case HK_RIGHT:
        t->cursor = te_caret_right(b, t->cursor);
        return;
    case HK_UP:
        if (allow_nl)
            t->cursor = te_caret_up(b, t->cursor);
        return;
    case HK_DOWN:
        if (allow_nl)
            t->cursor = te_caret_down(b, t->cursor);
        return;
    }
    if (k.c >= 0x20 && k.c < 0x110000) {
        char tmp[4];
        int n = u8_enc(k.c, tmp);
        t->cursor = te_insert(b, t->cursor, tmp, (size_t)n);
    }
}

static int motion(Text_Input *t, String_Builder *b, Key k, int allow_nl)
{
    size_t c = t->cursor;
    switch (k.c) {
    case 'h':
    case HK_LEFT:
        c = te_move_left(b, c);
        break;
    case 'l':
    case HK_RIGHT:
        c = te_move_right(b, c);
        break;
    case 'j':
    case HK_DOWN:
        if (!allow_nl)
            return 0;
        c = te_move_down(b, c);
        break;
    case 'k':
    case HK_UP:
        if (!allow_nl)
            return 0;
        c = te_move_up(b, c);
        break;
    case '0':
        c = te_line_start(b, c);
        break;
    case '$':
        c = te_line_end(b, c);
        break;
    case 'w':
        c = te_word_forward(b, c);
        break;
    case 'e':
        c = te_word_end(b, c);
        break;
    case 'b':
        c = te_word_back(b, c);
        break;
    default:
        return 0;
    }
    t->cursor = c;
    return 1;
}

static void visual_key(Text_Input *t, String_Builder *b, Key k, int allow_nl)
{
    if (k.c == 'v') {
        t->anchor = -1;
        return;
    }
    if (motion(t, b, k, allow_nl))
        return;
    size_t lo = (size_t)t->anchor < t->cursor ? (size_t)t->anchor : t->cursor;
    size_t hi = (size_t)t->anchor < t->cursor ? t->cursor : (size_t)t->anchor;
    switch (k.c) {
    case 'y':
        ti_set_clip(t, te_yank(b, lo, hi));
        t->cursor = lo;
        t->anchor = -1;
        break;
    case 'd':
    case 'x':
    case 'c':
        ti_set_clip(t, te_yank(b, lo, hi));
        t->cursor = te_delete_range(b, lo, hi);
        t->anchor = -1;
        t->insert = k.c == 'c';
        break;
    }
}

void ti_key(Text_Input *t, String_Builder *b, Key k, int allow_nl)
{
    if (t->insert) {
        insert_key(t, b, k, allow_nl);
        return;
    }
    if (t->pending_d) {
        t->pending_d = 0;
        if (k.c == 'd' && !k.ctrl) {
            t->cursor = te_delete_line(b, t->cursor);
            return;
        }
    }
    if (k.ctrl)
        return;
    if (t->anchor >= 0) {
        visual_key(t, b, k, allow_nl);
        return;
    }
    if (motion(t, b, k, allow_nl))
        return;
    const char *clip = t->clip ? t->clip : "";
    switch (k.c) {
    case 'd':
        t->pending_d = 1;
        break;
    case 'v':
        t->anchor = (long)t->cursor;
        break;
    case 'p':
        t->cursor = te_paste_after(b, t->cursor, clip);
        break;
    case 'P':
        t->cursor = te_paste_before(b, t->cursor, clip);
        break;
    case 'x':
        t->cursor = te_delete_char(b, t->cursor);
        break;
    case 'D':
        t->cursor = te_delete_to_line_end(b, t->cursor);
        break;
    case 'i':
        t->cursor = te_clamp_normal(b, t->cursor);
        t->insert = 1;
        break;
    case 'a':
        t->cursor = te_insert_after(b, t->cursor);
        t->insert = 1;
        break;
    case 'I':
        t->cursor = te_line_start(b, t->cursor);
        t->insert = 1;
        break;
    case 'A':
        t->cursor = te_insert_line_end(b, t->cursor);
        t->insert = 1;
        break;
    case 'o':
        t->cursor = te_open_below(b, t->cursor, allow_nl);
        t->insert = 1;
        break;
    case 'O':
        t->cursor = te_open_above(b, t->cursor, allow_nl);
        t->insert = 1;
        break;
    }
}
