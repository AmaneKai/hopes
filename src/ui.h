#include "hopes.h"
#include <locale.h>
#ifndef _WIN32
#include <langinfo.h>
#endif
#ifdef _WIN32
#undef MOUSE_MOVED
#include <curses.h>
#else
#include <ncurses.h>
#endif

typedef chtype At;

typedef struct {
    int x, y, w, h;
} Rect;

typedef struct {
    int x, y;
} Point;

enum {
    C_DEFAULT = -1,
    C_RED = 1,
    C_GREEN = 2,
    C_YELLOW = 3,
    C_BLUE = 4,
    C_MAGENTA = 5,
    C_CYAN = 6,
    C_GRAY = 7,
    C_DARK = 8,
    C_LIGHT_RED = 9
};

#define WIDE_COLS 115
#define MEDIUM_COLS 80
#define BAR_BG 236
#define KEY_ESCAPE 27

static WINDOW *win;
static int term_w, term_h, ncolors;
static bool color_ok;
static bool pair_ready[40];
static int bar_bg;

static Rect clip;
static bool dry;

static Point caret;
static bool caret_on;

static At fg(int color, bool bar)
{
    At extra = 0;
    int idx;

    if (!color_ok)
        return color == C_DARK ? A_DIM : A_NORMAL;
    if (color >= 8 && ncolors < 16) {
        color -= 8;
        extra = A_BOLD;
    }
    idx = color + 2 + (bar ? 18 : 0);
    if (!pair_ready[idx]) {
        init_pair((short)idx, (short)color, (short)(bar ? bar_bg : -1));
        pair_ready[idx] = true;
    }
    return COLOR_PAIR(idx) | extra;
}

static At at_muted(void)
{
    return fg(C_DARK, false);
}

static At at_section(void)
{
    return fg(C_DARK, false) | A_BOLD;
}

static At at_tag(void)
{
    return fg(C_CYAN, false);
}

static At at_prio(int p)
{
    static const int colors[] = {C_LIGHT_RED, C_RED, C_YELLOW, C_DARK};

    return fg(colors[p], false);
}

static At at_status(int s)
{
    static const int colors[] = {C_DARK, C_CYAN, C_GREEN};

    return fg(colors[s], false);
}

static void measure_screen(void)
{
#ifdef _WIN32
    term_w = COLS;
    term_h = LINES;
#else
    struct winsize ws;

    term_w = 80;
    term_h = 24;
    if (ioctl(1, TIOCGWINSZ, &ws) || !ws.ws_col || !ws.ws_row)
        return;
    term_w = ws.ws_col;
    term_h = ws.ws_row;
#endif
}

static void set_clip(int x, int y, int w, int h)
{
    clip = (Rect){x, y, w, h};
}

static void clip_screen(void)
{
    set_clip(0, 0, term_w, term_h);
}

static int putn(int y, int x, int maxx, At at, const char *s, size_t n)
{
    char out[256];
    size_t ol = 0;
    int start = x;

    if (y < clip.y || y >= clip.y + clip.h)
        return x;
    if (maxx > clip.x + clip.w)
        maxx = clip.x + clip.w;
    wattrset(win, at);
    for (size_t i = 0; i < n;) {
        uint32_t cp;
        int len = u8_dec(s + i, n - i, &cp), cw = u8_cp_width(cp);

        if (x + cw > maxx || ol + 4 > sizeof out)
            break;
        if (cp < 0x20 || cp == 0x7F) {
            out[ol++] = ' ';
        } else {
            memcpy(out + ol, s + i, (size_t)len);
            ol += (size_t)len;
        }
        x += cw;
        i += (size_t)len;
    }
    if (!dry && ol && start >= clip.x)
        mvwaddnstr(win, y, start, out, (int)ol);
    return x;
}

static int put(int y, int x, int maxx, At at, const char *s)
{
    return putn(y, x, maxx, at, s, strlen(s));
}

static void fill(int y, int x, int w, At at)
{
    char spaces[64];

    memset(spaces, ' ', sizeof spaces);
    while (w > 0) {
        int n = w < (int)sizeof spaces ? w : (int)sizeof spaces;

        putn(y, x, x + n, at, spaces, (size_t)n);
        x += n;
        w -= n;
    }
}

static void rule(int y, int x, int w, At at)
{
    for (int i = 0; i < w; i++)
        put(y, x + i, x + i + 1, at, "─");
}

static int text_width(const char *s)
{
    return u8_width(s, strlen(s));
}

static size_t wrap_chunk(const char *s, size_t len, int w)
{
    size_t i = 0, last_space = 0;
    uint32_t first;
    int cols = 0;

    while (i < len) {
        uint32_t cp;
        int n = u8_dec(s + i, len - i, &cp), cw = u8_cp_width(cp);

        if (cols + cw > w)
            break;
        cols += cw;
        i += (size_t)n;
        if (cp == ' ')
            last_space = i;
    }
    if (i == len)
        return len;
    if (last_space)
        return last_space;
    if (i)
        return i;
    return (size_t)u8_dec(s, len, &first);
}

static int put_wrap(int y, int x, int w, At at, const char *s)
{
    int rows = 0;

    if (w <= 0)
        return 0;
    for (;;) {
        const char *nl = strchr(s, '\n');
        size_t len = nl ? (size_t)(nl - s) : strlen(s), off = 0;

        do {
            size_t n = wrap_chunk(s + off, len - off, w);

            putn(y + rows, x, x + w, at, s + off, n);
            rows++;
            off += n;
        } while (off < len);
        if (!nl)
            return rows;
        s = nl + 1;
    }
}

static int measure(int w, const char *s)
{
    int rows;

    dry = true;
    rows = put_wrap(0, 0, w, A_NORMAL, s);
    dry = false;
    return rows;
}

static Rect pane(Rect r, const char *title, At title_at, bool focused)
{
    At border = focused ? A_NORMAL : at_muted();
    int x;

    set_clip(r.x, r.y, r.w, r.h);
    for (int i = 0; i < r.h; i++)
        put(r.y + i, r.x, r.x + 1, border, "│");
    x = put(r.y, r.x + 1, r.x + r.w, title_at, " ");
    put(r.y, x, r.x + r.w, title_at, title);
    set_clip(r.x + 2, r.y + 1, r.w > 3 ? r.w - 3 : 0, r.h > 1 ? r.h - 1 : 0);
    return (Rect){r.x + 2, r.y + 1, r.w > 3 ? r.w - 3 : 0, r.h > 1 ? r.h - 1 : 0};
}

static At pane_title(bool focused)
{
    return focused ? A_BOLD : at_section();
}

static Rect popup(const char *title, int w, int h)
{
    Rect r;
    int x;

    w = w < term_w - 2 ? w : term_w - 2;
    h = h < term_h - 2 ? h : term_h - 2;
    r = (Rect){(term_w - w) / 2, (term_h - h) / 2, w > 1 ? w : 1, h > 1 ? h : 1};
    clip_screen();
    for (int y = 0; y < r.h; y++)
        fill(r.y + y, r.x, r.w, A_NORMAL);
    put(r.y, r.x, r.x + r.w, at_muted(), "╭");
    put(r.y + r.h - 1, r.x, r.x + r.w, at_muted(), "╰");
    put(r.y, r.x + r.w - 1, r.x + r.w, at_muted(), "╮");
    put(r.y + r.h - 1, r.x + r.w - 1, r.x + r.w, at_muted(), "╯");
    for (int i = 1; i < r.w - 1; i++) {
        put(r.y, r.x + i, r.x + r.w, at_muted(), "─");
        put(r.y + r.h - 1, r.x + i, r.x + r.w, at_muted(), "─");
    }
    for (int y = 1; y < r.h - 1; y++) {
        put(r.y + y, r.x, r.x + r.w, at_muted(), "│");
        put(r.y + y, r.x + r.w - 1, r.x + r.w, at_muted(), "│");
    }
    x = put(r.y, r.x + 1, r.x + r.w - 1, A_BOLD, " ");
    x = put(r.y, x, r.x + r.w - 1, A_BOLD, title);
    put(r.y, x, r.x + r.w - 1, A_BOLD, " ");
    set_clip(r.x + 2, r.y + 1, r.w - 4, r.h - 2);
    return (Rect){r.x + 2, r.y + 1, r.w - 4, r.h - 2};
}

static const char *sync_text(At *at)
{
    *at = at_muted();
    switch (A.sync) {
    case SY_DISABLED: return NULL;
    case SY_IDLE: return "Idle";
    case SY_SYNCING:
        *at = fg(C_YELLOW, false);
        return A.sync_msg;
    case SY_SYNCED:
    case SY_UPDATED: return "Synced";
    case SY_OFFLINE: return "Offline";
    case SY_CONFLICT:
        *at = fg(C_RED, false) | A_BOLD;
        return "Conflict";
    default:
        *at = fg(C_RED, false) | A_BOLD;
        return "Sync error";
    }
}

static void draw_header(int y)
{
    const char *view = A.view == V_TABLE ? "Tasks" : "Board";
    char current[96], right[64], count[32];
    At sync_at;
    const char *sync = sync_text(&sync_at);
    int x, rw;

    if (!A.filter)
        snprintf(current, sizeof current, "%s", nav_label(A.section));
    else
        snprintf(current, sizeof current, "%s%s", A.filter[0] == '#' ? "" : "/", A.filter);

    x = put(y, 0, term_w, A_NORMAL, "  ");
    x = put(y, x, term_w, at_muted(), "Hopes / ");
    x = put(y, x, term_w, at_muted(), view);
    x = put(y, x, term_w, at_muted(), " / ");
    x = put(y, x, term_w, A_BOLD, current);
    if (A.filter) {
        snprintf(count, sizeof count, "  %d/%d", len_of(A.vis), A.stats.total);
        put(y, x, term_w, at_muted(), count);
    }

    snprintf(right, sizeof right, "%d%% done", A.stats.percent);
    rw = text_width(right) + 2;
    if (sync)
        rw += 3 + 2 + text_width(sync);
    x = term_w - rw;
    x = put(y, x, term_w, at_muted(), right);
    if (!sync)
        return;
    x = put(y, x, term_w, A_NORMAL, "   ");
    x = put(y, x, term_w, sync_at, "☁ ");
    put(y, x, term_w, sync_at, sync);
}

static void nav_row(int y, Rect r, const char *prefix, const char *label, At at, int count)
{
    char num[16];
    int x = put(y, r.x, r.x + r.w, A_NORMAL, prefix), gap;

    snprintf(num, sizeof num, "%d", count);
    x = put(y, x, r.x + r.w, at, label);
    gap = r.x + r.w - x - (int)strlen(num);
    put(y, x + (gap > 1 ? gap : 1), r.x + r.w, at_muted(), num);
}

static const char *cursor_prefix(bool is_cursor, bool is_current)
{
    return is_cursor ? "> " : is_current ? "● " : "  ";
}

static int nav_count(int section)
{
    switch (section) {
    case NAV_ALL: return A.stats.total;
    case NAV_TODO: return A.stats.todo;
    case NAV_WIP: return A.stats.wip;
    case NAV_DONE: return A.stats.done;
    default: return A.stats.prio[P_URGENT];
    }
}

static void draw_nav_tags(int y, Rect r, bool focused)
{
    int shown = len_of(A.tags) < NAV_TAGS_SHOWN ? len_of(A.tags) : NAV_TAGS_SHOWN;

    if (!shown)
        put(y, r.x, r.x + r.w, at_muted(), "  No tags yet");
    for (int i = 0; i < shown; i++) {
        const char *name = A.tags[i].name;
        bool current = A.filter && !strcmp(A.filter[0] == '#' ? A.filter + 1 : A.filter, name);
        bool cursor = focused && A.nav_idx == NAV_COUNT + i;
        At at = current || cursor ? fg(C_CYAN, false) | A_BOLD : at_tag();
        char label[96];

        snprintf(label, sizeof label, "#%s", name);
        nav_row(y + i, r, cursor_prefix(cursor, current), label, at, A.tags[i].count);
    }
}

static void draw_nav(Rect area, bool focused)
{
    Rect r = pane(area, "Navigator", pane_title(focused), focused);
    int y = r.y + 1;

    put(y++, r.x, r.x + r.w, at_section(), "  Views");
    for (int i = 0; i < NAV_COUNT; i++) {
        bool current = (int)A.section == i && !A.filter, cursor = focused && A.nav_idx == i;

        nav_row(y++, r, cursor_prefix(cursor, current), nav_label(i), current || cursor ? A_BOLD : A_NORMAL,
                nav_count(i));
    }
    y++;
    put(y++, r.x, r.x + r.w, at_section(), "  Priority");
    for (int p = 0; p < 4; p++) {
        char prefix[16];

        snprintf(prefix, sizeof prefix, "%s ", prio_glyph(p));
        put(y, r.x, r.x + r.w, at_prio(p), prefix);
        nav_row(y++, (Rect){r.x + 2, r.y, r.w - 2, r.h}, "", prio_label(p), A_NORMAL, A.stats.prio[p]);
    }
    y++;
    put(y++, r.x, r.x + r.w, at_section(), "  Tags");
    draw_nav_tags(y, r, focused);
}

static void table_scroll(int rows)
{
    int n = len_of(A.vis);

    if (A.tbl_sel >= 0 && A.tbl_sel < A.tbl_off)
        A.tbl_off = A.tbl_sel;
    if (A.tbl_sel >= A.tbl_off + rows && rows > 0)
        A.tbl_off = A.tbl_sel - rows + 1;
    if (A.tbl_off > n - rows)
        A.tbl_off = n > rows ? n - rows : 0;
}

static void draw_table_row(int y, Rect r, int row, int title_w, int tag_w)
{
    const Item *it = &A.items[A.vis[row]];
    bool sel = row == A.tbl_sel, done = it->status == ST_DONE;
    At bold = sel ? A_BOLD : A_NORMAL;
    char prio[24], tags[160] = "";
    int x = r.x + 2;

    put(y, r.x, r.x + r.w, bold, sel ? "> " : "  ");
    put(y, x, x + 2, at_status(it->status) | bold, status_badge(it->status));
    x += 3;
    put(y, x, x + title_w, (done ? at_muted() : A_NORMAL) | bold, it->title);
    x += title_w + 1;
    if (!done) {
        snprintf(prio, sizeof prio, "%s %s", prio_glyph(it->prio), prio_short(it->prio));
        put(y, x, x + 7, at_prio(it->prio) | (it->prio <= P_HIGH ? A_BOLD : 0) | bold, prio);
    }
    x += 8;
    for (int t = 0; t < len_of(it->tags) && strlen(tags) < sizeof tags - 40; t++)
        snprintf(tags + strlen(tags), sizeof tags - strlen(tags), "%s#%s", t ? " " : "", it->tags[t]);
    put(y, x, x + tag_w, at_tag() | bold, tags);
}

static void draw_table(Rect area, bool focused)
{
    char title[160];
    Rect r;
    int avail, tag_w, title_w, n = len_of(A.vis);

    if (A.filter)
        snprintf(title, sizeof title, "Tasks · %s · %d", A.filter, n);
    else
        snprintf(title, sizeof title, "Tasks · %d", n);
    r = pane(area, title, pane_title(focused), focused);
    if (!n) {
        const char *msg = A.items ? "No tasks match. Press <Esc> to clear the filter."
                                  : "Nothing here yet. Press 'a' to add a task.";
        int pad = (r.w - text_width(msg)) / 2;

        put(r.y, r.x + (pad > 0 ? pad : 0), r.x + r.w, at_muted(), msg);
        return;
    }
    avail = r.w - 2;
    tag_w = avail * 25 / 100;
    title_w = avail - 2 - 7 - tag_w - 3;
    if (title_w < 12 && tag_w > 12 - title_w)
        tag_w -= 12 - title_w, title_w = 12;
    if (title_w < 1)
        title_w = 1;
    table_scroll(r.h);
    for (int i = 0; i < r.h && A.tbl_off + i < n; i++)
        draw_table_row(r.y + i, r, A.tbl_off + i, title_w, tag_w);
}

static int card_height(const Item *it, int w)
{
    return measure(w - 4, it->title) + (it->desc[0] != 0) + (len_of(it->tags) > 0) + 1;
}

static void kanban_scroll(int col, int rows, int w)
{
    int top = 0, sel = A.kb_cur[col];

    for (int i = 0; i < sel && i < len_of(A.col[col]); i++)
        top += card_height(&A.items[A.col[col][i]], w);
    if (sel < len_of(A.col[col])) {
        int h = card_height(&A.items[A.col[col][sel]], w);

        if (top < A.kb_off[col])
            A.kb_off[col] = top;
        if (top + h > A.kb_off[col] + rows)
            A.kb_off[col] = top + h - rows;
    }
}

static int draw_card(int y, Rect r, const Item *it, bool selected)
{
    char desc[64], tags[160] = "";
    int rows, x = r.x;

    put(y, x, x + 2, A_NORMAL, selected ? "> " : "  ");
    put(y, x + 2, x + 4, at_prio(it->prio), prio_glyph(it->prio));
    rows = put_wrap(y, x + 4, r.w - 4,
                    it->status == ST_DONE ? at_muted() : selected ? A_BOLD : A_NORMAL, it->title);
    if (it->desc[0]) {
        snippet(desc, sizeof desc, it->desc, 32);
        put(y + rows++, x + 4, x + r.w, at_muted(), desc);
    }
    for (int t = 0; t < len_of(it->tags) && strlen(tags) < sizeof tags - 40; t++)
        snprintf(tags + strlen(tags), sizeof tags - strlen(tags), "#%s ", it->tags[t]);
    if (tags[0])
        put(y + rows++, x + 4, x + r.w, at_tag(), tags);
    return rows + 1;
}

static void draw_kanban_column(Rect area, int col, bool active)
{
    static const char *const names[] = {"To Do", "In Progress", "Completed"};
    static const char *const empty[] = {"Nothing to do", "Nothing in progress", "Nothing completed yet"};
    static const int colors[] = {C_DEFAULT, C_CYAN, C_GREEN};
    char title[64];
    Rect r;
    int y, n = len_of(A.col[col]);

    snprintf(title, sizeof title, "%s · %d", names[col], n);
    r = pane(area, title, fg(colors[col], false) | A_BOLD, active);
    if (!n) {
        put(r.y, r.x, r.x + r.w, at_muted(), empty[col]);
        return;
    }
    kanban_scroll(col, r.h, r.w);
    y = r.y - A.kb_off[col];
    for (int i = 0; i < n; i++)
        y += draw_card(y, r, &A.items[A.col[col][i]], active && i == A.kb_cur[col]);
}

static void draw_kanban(Rect area, bool focused)
{
    int w1 = area.w * 33 / 100, w2 = area.w * 34 / 100;
    Rect cols[3] = {
        {area.x, area.y, w1, area.h},
        {area.x + w1, area.y, w2, area.h},
        {area.x + w1 + w2, area.y, area.w - w1 - w2, area.h},
    };

    for (int c = 0; c < 3; c++)
        draw_kanban_column(cols[c], c, focused && A.kb_col == c);
}

static void property(int y, Rect r, const char *name, At at, const char *value)
{
    char key[16];
    int x;

    snprintf(key, sizeof key, "%-11s", name);
    x = put(y, r.x, r.x + r.w, at_muted(), key);
    put(y, x, r.x + r.w, at, value);
}

static int draw_details_body(int y, Rect r, const Item *it)
{
    char tags[256] = "", line[64];
    int y0 = y;

    y += put_wrap(y, r.x, r.w, A_BOLD, it->title) + 1;
    snprintf(line, sizeof line, "%s %s", status_badge(it->status), status_label(it->status));
    property(y++, r, "Status", at_status(it->status), line);
    snprintf(line, sizeof line, "%s %s", prio_glyph(it->prio), prio_label(it->prio));
    property(y++, r, "Priority", at_prio(it->prio), line);
    for (int t = 0; t < len_of(it->tags) && strlen(tags) < sizeof tags - 40; t++)
        snprintf(tags + strlen(tags), sizeof tags - strlen(tags), "%s#%s", t ? ", " : "", it->tags[t]);
    property(y++, r, "Tags", tags[0] ? at_tag() : at_muted(), tags[0] ? tags : "Empty");
    if (it->desc[0]) {
        rule(y++, r.x, r.w, at_muted());
        y += put_wrap(y, r.x, r.w, A_NORMAL, it->desc);
    }
    return y - y0;
}

static void draw_details(Rect area, bool focused)
{
    Rect r = pane(area, "Inspector", pane_title(focused), focused);
    int sel = app_selected();

    A.insp_vis = r.h;
    if (sel < 0) {
        put(r.y + 1, r.x, r.x + r.w, at_muted(), "No task selected");
        A.insp_total = 0;
        return;
    }
    A.insp_total = 1 + draw_details_body(r.y + 1 - A.insp_scroll, r, &A.items[sel]);
}

static void draw_progress(Rect area)
{
    enum { BAR = 16 };
    Rect r = pane(area, "Progress", pane_title(false), false);
    int filled = A.stats.percent * BAR / 100, x;
    char line[96];

    x = put(r.y, r.x, r.x + r.w, at_muted(), "Done       ");
    for (int i = 0; i < BAR; i++)
        x = put(r.y, x, r.x + r.w, i < filled ? fg(C_GREEN, false) : at_muted(), "━");
    snprintf(line, sizeof line, " %d%%", A.stats.percent);
    put(r.y, x, r.x + r.w, at_muted(), line);

    snprintf(line, sizeof line, "%d to do · %d active · %d done", A.stats.todo, A.stats.wip, A.stats.done);
    property(r.y + 1, r, "Status", A_NORMAL, line);
    x = put(r.y + 2, r.x, r.x + r.w, at_muted(), "Priority   ");
    for (int p = 0; p < 4; p++) {
        snprintf(line, sizeof line, "%s ", prio_glyph(p));
        x = put(r.y + 2, x, r.x + r.w, at_prio(p), line);
        snprintf(line, sizeof line, "%d  ", A.stats.prio[p]);
        x = put(r.y + 2, x, r.x + r.w, A_NORMAL, line);
    }
}

static void draw_activity(Rect area)
{
    Rect r = pane(area, "Activity", pane_title(false), false);

    if (!A.naudit)
        put(r.y, r.x, r.x + r.w, at_muted(), "No activity yet this session");
    for (int i = 0; i < A.naudit && i < r.h; i++) {
        char stamp[16];
        struct tm tm;
        int x;

        plat_localtime(A.audit[i].at, &tm);
        strftime(stamp, sizeof stamp, "%H:%M ", &tm);
        x = put(r.y + i, r.x, r.x + r.w, at_muted(), stamp);
        put(r.y + i, x, r.x + r.w, A_NORMAL, A.audit[i].msg);
    }
}

static void draw_inspector(Rect area, bool focused)
{
    int fixed = 6 + 8, details = area.h - fixed, progress, activity;

    if (details < 8)
        details = area.h < 8 ? area.h : 8;
    progress = area.h - details < 6 ? area.h - details : 6;
    activity = area.h - details - progress;
    draw_details((Rect){area.x, area.y, area.w, details}, focused);
    if (progress > 0)
        draw_progress((Rect){area.x, area.y + details, area.w, progress});
    if (activity > 0)
        draw_activity((Rect){area.x, area.y + details + progress, area.w, activity});
}

static void draw_field_line(int y, Rect r, const char *s, size_t len, int left, long lo, long hi, long base)
{
    int col = 0;

    for (size_t i = 0; i < len;) {
        uint32_t cp;
        int n = u8_dec(s + i, len - i, &cp), cw = u8_cp_width(cp);
        At at = (long)i + base >= lo && (long)i + base < hi ? A_REVERSE : A_NORMAL;

        if (col >= left && col - left + cw > r.w)
            break;
        if (col >= left)
            putn(y, r.x + col - left, r.x + r.w, at, s + i, (size_t)n);
        col += cw;
        i += (size_t)n;
    }
}

static Point caret_cell(const char *s, size_t cursor)
{
    Point p = {0, 0};

    for (size_t i = 0; i < cursor && s[i];) {
        uint32_t cp;

        i += (size_t)u8_dec(s + i, strlen(s + i), &cp);
        if (cp == '\n')
            p = (Point){0, p.y + 1};
        else
            p.x += u8_cp_width(cp);
    }
    return p;
}

static void draw_field(Rect r, const String_Builder *b, const Text_Input *in, bool focused, const char *placeholder)
{
    const char *s = sb_cstr(b);
    Point c = caret_cell(s, in->cursor);
    int top = c.y >= r.h ? c.y - r.h + 1 : 0, left = c.x >= r.w ? c.x - r.w + 1 : 0;
    long lo = -1, hi = -1;
    size_t off = 0;

    if (!b->count) {
        put(r.y, r.x, r.x + r.w, at_muted(), placeholder);
    }
    if (focused && in->anchor >= 0) {
        lo = (long)(in->cursor < (size_t)in->anchor ? in->cursor : (size_t)in->anchor);
        hi = (long)u8_next(s, b->count, in->cursor > (size_t)in->anchor ? in->cursor : (size_t)in->anchor);
    }
    for (int line = 0; line < top + r.h && off <= b->count; line++) {
        const char *nl = strchr(s + off, '\n');
        size_t len = nl ? (size_t)(nl - s) - off : b->count - off;

        if (line >= top)
            draw_field_line(r.y + line - top, r, s + off, len, left, lo, hi, (long)off);
        off += len + 1;
    }
    if (focused) {
        caret = (Point){r.x + c.x - left, r.y + c.y - top};
        caret_on = true;
    }
}

static void split_label(Rect row, const char *label, bool focused, Rect *value)
{
    set_clip(row.x, row.y, row.w, row.h);
    put(row.y, row.x, row.x + 2, fg(C_CYAN, false), focused ? "▍" : " ");
    put(row.y, row.x + 2, row.x + 12, focused ? A_BOLD : at_muted(), label);
    *value = (Rect){row.x + 12, row.y, row.w - 12, row.h};
}

static void draw_editor_priority(Rect row, bool focused)
{
    Rect v;
    int x;

    split_label(row, "Priority", focused, &v);
    x = v.x;
    for (int p = 0; p < 4; p++) {
        char label[40];

        snprintf(label, sizeof label, "%s %s   ", prio_glyph(p), prio_label(p));
        x = put(row.y, x, v.x + v.w, at_prio(p) | (p == A.ed.prio ? A_BOLD | A_UNDERLINE : 0), label);
    }
}

static void draw_editor(void)
{
    Editor *e = &A.ed;
    Rect in = popup(e->edit_idx >= 0 ? "Edit task" : "New task", 68, 14), v;

    split_label((Rect){in.x, in.y + 1, in.w, 1}, "Title", e->field == FF_TITLE, &v);
    draw_field(v, &e->title, &e->in, e->field == FF_TITLE, "Untitled");
    rule(in.y + 2, in.x, in.w, e->field == FF_TITLE ? A_NORMAL : at_muted());

    split_label((Rect){in.x, in.y + 3, in.w, 3}, "Notes", e->field == FF_DESC, &v);
    draw_field(v, &e->desc, &e->in, e->field == FF_DESC, "Add notes...");
    rule(in.y + 6, in.x, in.w, e->field == FF_DESC ? A_NORMAL : at_muted());

    draw_editor_priority((Rect){in.x, in.y + 8, in.w, 1}, e->field == FF_PRIO);
    split_label((Rect){in.x, in.y + 9, in.w, 1}, "Tags", e->field == FF_TAGS, &v);
    draw_field(v, &e->tags, &e->in, e->field == FF_TAGS, "design, ui");

    set_clip(in.x, in.y, in.w, in.h);
    {
        int x = put(in.y + in.h - 1, in.x, in.x + in.w, A_BOLD, "[Ctrl-s] ");

        x = put(in.y + in.h - 1, x, in.x + in.w, at_muted(), "save   ");
        x = put(in.y + in.h - 1, x, in.x + in.w, A_BOLD, "[Tab] ");
        x = put(in.y + in.h - 1, x, in.x + in.w, at_muted(), "next   ");
        x = put(in.y + in.h - 1, x, in.x + in.w, A_BOLD, "[Esc] ");
        put(in.y + in.h - 1, x, in.x + in.w, at_muted(), "cancel");
    }
}

typedef struct {
    const char *keys, *desc;
} Help_Row;

static const Help_Row HELP[] = {
    {"Navigation", ""},
    {"j / Down", "Move down"},
    {"k / Up", "Move up"},
    {"gg / Home", "Jump to top"},
    {"G / End", "Jump to bottom"},
    {"Ctrl-d / Ctrl-u", "Page down / up"},
    {"Tab", "Next pane"},
    {"v", "Switch table / board"},
    {"s", "Cycle sort"},
    {"Search", ""},
    {"/", "Find tasks"},
    {"t", "Cycle tag filter"},
    {"Esc", "Clear filter"},
    {":", "Command line"},
    {"Tasks", ""},
    {"Space", "Toggle status"},
    {"a / o / i", "Add task"},
    {"e / c / Enter", "Edit task"},
    {"dd / x", "Delete task"},
    {"u", "Undo delete"},
    {"J / K", "Move task down / up"},
    {"p / P", "Next / previous priority"},
    {"1 2 3 4", "Urgent / High / Medium / Low"},
    {"S / F5", "Sync now"},
    {"Editing", ""},
    {"Tab / Ctrl-j", "Next field"},
    {"Shift-Tab / Ctrl-k", "Previous field"},
    {"Ctrl-s", "Save"},
    {"Esc", "Normal mode, then cancel"},
    {"h j k l w b e", "Move"},
    {"0 / $", "Line start / end"},
    {"i a I A o", "Insert"},
    {"v", "Visual selection"},
    {"x dd D", "Delete"},
    {"y p P", "Yank / paste"},
    {"Ctrl-w / Ctrl-u", "Delete word / clear"},
    {"General", ""},
    {"? / q / Esc", "Close help"},
    {"Ctrl-c", "Cancel form"},
};

#define HELP_ROW_COUNT ((int)(sizeof HELP / sizeof *HELP))

static bool is_help_heading(const Help_Row *row)
{
    return !row->desc[0];
}

static int help_lines(void)
{
    int n = HELP_ROW_COUNT;

    for (int i = 1; i < HELP_ROW_COUNT; i++)
        n += is_help_heading(&HELP[i]);
    return n;
}

static void draw_help_row(int y, Rect r, const Help_Row *row)
{
    char keys[24];
    int x;

    if (is_help_heading(row)) {
        put(y, r.x, r.x + r.w, at_section(), row->keys);
        return;
    }
    snprintf(keys, sizeof keys, "%-16s", row->keys);
    x = put(y, r.x, r.x + r.w, A_BOLD, keys);
    put(y, x, r.x + r.w, at_muted(), row->desc);
}

static void draw_help(void)
{
    int total = help_lines(), line = 0;
    Rect in = popup("Keyboard shortcuts", 64, total + 4), body;

    body = (Rect){in.x, in.y, in.w, in.h - 2};
    A.help_max = total > body.h ? total - body.h : 0;
    if (A.help_scroll > A.help_max)
        A.help_scroll = A.help_max;
    set_clip(body.x, body.y, body.w, body.h);
    for (int i = 0; i < HELP_ROW_COUNT; i++) {
        line += i > 0 && is_help_heading(&HELP[i]);
        draw_help_row(body.y + line++ - A.help_scroll, body, &HELP[i]);
    }
    set_clip(in.x, in.y, in.w, in.h);
    {
        int x = put(in.y + in.h - 1, in.x, in.x + in.w, A_BOLD, "[j/k] ");

        x = put(in.y + in.h - 1, x, in.x + in.w, at_muted(), "scroll   ");
        x = put(in.y + in.h - 1, x, in.x + in.w, A_BOLD, "[Esc] ");
        put(in.y + in.h - 1, x, in.x + in.w, at_muted(), "close");
    }
}

static const Text_Input *active_input(void)
{
    if (A.mode == M_CREATE || A.mode == M_EDIT)
        return A.ed.field == FF_PRIO ? NULL : &A.ed.in;
    if (A.mode == M_FIND || A.mode == M_CMD)
        return &A.cmd.in;
    return NULL;
}

static void draw_status_line(int y)
{
    static const char *const labels[] = {"NORMAL", "INSERT", "EDIT", "FIND", "COMMAND", "HELP"};
    static const int colors[] = {C_GRAY, C_CYAN, C_MAGENTA, C_YELLOW, C_DEFAULT, C_BLUE};
    const Text_Input *in = active_input();
    const char *pending = app_chord_label();
    char pos[48], text[32];
    int x, rw;

    fill(y, 0, term_w, fg(C_DEFAULT, true));
    snprintf(text, sizeof text, "  %s", labels[A.mode]);
    x = put(y, 0, term_w, fg(colors[A.mode], true) | A_BOLD, text);
    x = put(y, x, term_w, fg(C_DEFAULT, true), " ");
    if (in) {
        const char *badge = in->insert ? "-- INSERT --" : in->anchor >= 0 ? "-- VISUAL --" : "-- NORMAL --";
        int color = in->insert ? C_GREEN : in->anchor >= 0 ? C_MAGENTA : C_YELLOW;

        x = put(y, x, term_w, fg(color, true) | A_BOLD, badge);
        x = put(y, x, term_w, fg(C_DEFAULT, true), " ");
    }
    if (pending) {
        x = put(y, x, term_w, fg(C_YELLOW, true), pending);
        x = put(y, x, term_w, fg(C_DEFAULT, true), " ");
    }
    if (A.status[0] && now_ms() < A.status_until)
        put(y, x, term_w, fg(C_YELLOW, true), A.status);

    snprintf(pos, sizeof pos, "%d / %d  ", app_position(),
             A.view == V_KANBAN ? len_of(A.col[A.kb_col]) : len_of(A.vis));
    rw = text_width(pos);
    put(y, term_w - rw, term_w, fg(C_DARK, true), pos);
}

typedef struct {
    const char *key, *desc;
} Hint;

static int hint_width(const Hint *h, int n)
{
    int w = 0;

    for (int i = 0; i < n; i++)
        w += text_width(h[i].key) + text_width(h[i].desc) + 5;
    return w;
}

static void draw_hints_right(int y, const Hint *h, int n)
{
    int x = term_w - hint_width(h, n);

    for (int i = 0; i < n; i++) {
        char key[40];

        snprintf(key, sizeof key, "[%s] ", h[i].key);
        x = put(y, x, term_w, A_BOLD, key);
        x = put(y, x, term_w, at_muted(), h[i].desc);
        x = put(y, x, term_w, A_NORMAL, "  ");
    }
}

static void draw_hints_inline(int y, const Hint *h, int n)
{
    int x = 0;

    for (int i = 0; i < n; i++) {
        x = put(y, x, term_w, A_BOLD, h[i].key);
        x = put(y, x, term_w, at_muted(), h[i].desc);
    }
}

static void draw_normal_hints(int y)
{
    static const Hint main_hints[] = {{"e", "edit"}, {"Space", "toggle"}, {"dd", "delete"}};
    static const Hint nav_hints[] = {{"Enter", "open"}, {"1-5", "jump"}};
    static const Hint insp_hints[] = {{"j/k", "scroll"}, {"Tab", "next pane"}};
    const Hint *ctx = A.focus == F_MAIN ? main_hints : A.focus == F_NAV ? nav_hints : insp_hints;
    int n = A.focus == F_MAIN ? 3 : 2, x;

    x = put(y, 0, term_w, A_BOLD, "  ? ");
    x = put(y, x, term_w, at_muted(), "help   ");
    x = put(y, x, term_w, A_BOLD, ": ");
    x = put(y, x, term_w, at_muted(), "commands   ");
    x = put(y, x, term_w, A_BOLD, "/ ");
    x = put(y, x, term_w, at_muted(), "find");
    if (term_w - x >= hint_width(ctx, n))
        draw_hints_right(y, ctx, n);
}

static void draw_edit_hints(int y)
{
    static const Hint visual[] = {{"h/j/k/l/w/b/e/0/$", ":Extend "}, {"d/x", ":Cut "}, {"y", ":Yank "},
                                  {"c", ":Change "}, {"v / <Esc>", ":Exit Visual"}};
    static const Hint normal[] = {{"h/j/k/l", ":Move "}, {"w/b/e", ":Word "}, {"0/$", ":Line "},
                                  {"v", ":Visual "}, {"x/dd/D", ":Delete "}, {"p/P", ":Paste "},
                                  {"i/a/I/A/o", ":Insert "}, {"<Esc>", ":Cancel"}};
    static const Hint insert[] = {{"<Tab>/<C-j/k>", ":Next Field "}, {"<Enter>/<C-s>", ":Save "},
                                  {"<C-w>", ":Del Word "}, {"<C-u>", ":Clear "},
                                  {"<Esc>", ":Normal Mode / Cancel"}};
    const Text_Input *in = active_input();

    if (in && in->anchor >= 0)
        draw_hints_inline(y, visual, 5);
    else if (in && !in->insert)
        draw_hints_inline(y, normal, 8);
    else
        draw_hints_inline(y, insert, 5);
}

static void draw_hints(int y)
{
    static const Hint help[] = {{"<Esc> / ? / q / <Enter>", ":Close Cheatsheet"}};

    switch (A.mode) {
    case M_NORMAL: draw_normal_hints(y); break;
    case M_CREATE:
    case M_EDIT: draw_edit_hints(y); break;
    case M_HELP: draw_hints_inline(y, help, 1); break;
    default: break;
    }
}

static void draw_cmdline(int y)
{
    Rect r = {1, y, term_w - 1, 1};

    fill(y, 0, term_w, A_NORMAL);
    put(y, 0, 1, A_NORMAL, A.cmd.find ? "/" : ":");
    draw_field(r, &A.cmd.buf, &A.cmd.in, true, "");
}

static void draw_body(Rect body)
{
    bool wide = body.w >= WIDE_COLS, medium = body.w >= MEDIUM_COLS;
    int side = wide ? body.w * 18 / 100 : 0;
    int insp = wide ? body.w * 34 / 100 : medium ? body.w * 40 / 100 : 0;
    Rect main_area = {body.x + side, body.y, body.w - side - insp, body.h};

    if (wide)
        draw_nav((Rect){body.x, body.y, side, body.h}, A.focus == F_NAV);
    if (A.view == V_TABLE)
        draw_table(main_area, A.focus == F_MAIN);
    else
        draw_kanban(main_area, A.focus == F_MAIN);
    if (insp)
        draw_inspector((Rect){body.x + body.w - insp, body.y, insp, body.h}, A.focus == F_INSPECTOR);
}

void ui_render(void)
{
    caret_on = false;
    measure_screen();
    werase(win);
    if (term_h < 6 || term_w < 20) {
        mvwaddstr(win, 0, 0, "terminal too small");
        wrefresh(win);
        return;
    }
    clip_screen();
    draw_header(0);
    rule(1, 0, term_w, at_muted());
    draw_body((Rect){0, 2, term_w, term_h - 4});
    clip_screen();
    draw_status_line(term_h - 2);
    if (A.mode == M_FIND || A.mode == M_CMD)
        draw_cmdline(term_h - 1);
    else
        draw_hints(term_h - 1);
    if (A.mode == M_CREATE || A.mode == M_EDIT)
        draw_editor();
    if (A.mode == M_HELP)
        draw_help();
    curs_set(caret_on);
    if (caret_on)
        wmove(win, caret.y, caret.x);
    wrefresh(win);
}

void ui_init(void)
{
    setlocale(LC_ALL, "");
#ifndef _WIN32
    if (!strstr(nl_langinfo(CODESET), "UTF-8"))
        setlocale(LC_CTYPE, "C.UTF-8");
#endif
    win = initscr();
    raw();
    noecho();
    nonl();
    keypad(win, TRUE);
    nodelay(win, TRUE);
#ifndef PDCURSES
    set_escdelay(25);
#endif
    curs_set(0);
    color_ok = has_colors();
    bar_bg = -1;
    if (!color_ok)
        return;
    start_color();
    use_default_colors();
#ifdef _WIN32
    ncolors = COLORS;
#else
    ncolors = tigetnum("colors");
#endif
    if (ncolors >= 256)
        bar_bg = BAR_BG;
}

void ui_end(void)
{
    endwin();
}

static bool map_special(int code, Key *k)
{
    static const struct {
        int from;
        uint32_t to;
    } table[] = {
        {KEY_UP, HK_UP},      {KEY_DOWN, HK_DOWN},   {KEY_LEFT, HK_LEFT},   {KEY_RIGHT, HK_RIGHT},
        {KEY_HOME, HK_HOME},  {KEY_END, HK_END},     {KEY_PPAGE, HK_PGUP},  {KEY_NPAGE, HK_PGDN},
        {KEY_BTAB, HK_BACKTAB}, {KEY_BACKSPACE, HK_BACKSPACE}, {KEY_ENTER, HK_ENTER},
        {KEY_F(5), HK_F5},
    };

    for (size_t i = 0; i < sizeof table / sizeof *table; i++) {
        if (table[i].from != code)
            continue;
        k->c = table[i].to;
        return true;
    }
    return false;
}

static bool map_char(wint_t ch, Key *k)
{
    k->c = ch;
    if (ch == KEY_ESCAPE || ch == '\r' || ch == '\t' || ch == 8 || ch == 127) {
        k->c = ch == KEY_ESCAPE ? HK_ESC : ch == '\r' ? HK_ENTER : ch == '\t' ? HK_TAB : HK_BACKSPACE;
        return true;
    }
    if (ch >= 1 && ch <= 26) {
        k->c = 'a' + ch - 1;
        k->ctrl = true;
    }
    return true;
}

bool ui_read_input(void)
{
    wint_t ch;
    int rc;

    while ((rc = wget_wch(win, &ch)) != ERR) {
        Key k = {0, false};
        bool ok;

#ifdef PDCURSES
        if (rc == KEY_CODE_YES && (int)ch == KEY_RESIZE)
            resize_term(0, 0);
#endif
        ok = rc == KEY_CODE_YES ? map_special((int)ch, &k) : map_char(ch, &k);

        if (ok)
            app_key(k);
        if (A.quit)
            return true;
    }
    return true;
}
