#include "hopes.h"

App A;

#define PAGE 5
#define STATUS_TTL_MS 4000

static const char *const NAV_LABEL[] = {"All", "To Do", "In Progress", "Completed", "Urgent"};
const char *nav_label(int s) { return NAV_LABEL[s]; }

void app_set_status(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(A.status, sizeof A.status, fmt, ap);
    va_end(ap);
    A.status_until = now_ms() + STATUS_TTL_MS;
}

void app_log(const char *fmt, ...)
{
    if (A.naudit == AUDIT_MAX)
        A.naudit--;
    memmove(&A.audit[1], &A.audit[0], sizeof(Audit_Entry) * (size_t)A.naudit);
    A.naudit++;
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(A.audit[0].msg, sizeof A.audit[0].msg, fmt, ap);
    va_end(ap);
    A.audit[0].at = time(NULL);
}

static bool ci_has(const char *h, const char *n, size_t nl)
{
    if (!nl)
        return true;
    for (; *h; h++) {
        size_t i = 0;
        while (i < nl && h[i] && (h[i] | 0x20) == (n[i] | 0x20) &&
               (((h[i] | 0x20) >= 'a' && (h[i] | 0x20) <= 'z') ? 1 : h[i] == n[i]))
            i++;
        if (i == nl)
            return true;
    }
    return false;
}

static bool matches_query(const Item *it, const char *filter)
{
    while (*filter == ' ' || *filter == '\t')
        filter++;
    size_t n = strlen(filter);
    while (n && (filter[n - 1] == ' ' || filter[n - 1] == '\t'))
        n--;
    if (!n)
        return true;
    if (filter[0] == '#') {
        for (int i = 0; i < len_of(it->tags); i++)
            if (ci_has(it->tags[i], filter + 1, n - 1))
                return true;
        return false;
    }
    if (ci_has(it->title, filter, n) || ci_has(it->desc, filter, n) ||
        ci_has(prio_label(it->prio), filter, n) || ci_has(status_label(it->status), filter, n))
        return true;
    for (int i = 0; i < len_of(it->tags); i++)
        if (ci_has(it->tags[i], filter, n))
            return true;
    return false;
}

static bool in_section(const Item *it)
{
    switch (A.section) {
    case NAV_TODO: return it->status == ST_TODO;
    case NAV_WIP: return it->status == ST_WIP;
    case NAV_DONE: return it->status == ST_DONE;
    case NAV_URGENT: return it->prio == P_URGENT;
    default: return true;
    }
}

static int cmp_prio(const void *a, const void *b)
{
    int x = *(const int *)a, y = *(const int *)b;
    int d = A.items[x].prio - A.items[y].prio;
    return d ? d : x - y;
}

static int cmp_status(const void *a, const void *b)
{
    int x = *(const int *)a, y = *(const int *)b;
    int d = A.items[x].status - A.items[y].status;
    return d ? d : x - y;
}

static int cmp_title(const void *a, const void *b)
{
    int x = *(const int *)a, y = *(const int *)b;
    int d = strcasecmp(A.items[x].title, A.items[y].title);
    return d ? d : x - y;
}

static int cmp_tag(const void *a, const void *b)
{
    const Tag_Count *x = a, *y = b;
    return x->count != y->count ? y->count - x->count : strcmp(x->name, y->name);
}

static void count_tag(const char *name)
{
    for (int k = 0; k < len_of(A.tags); k++) {
        if (strcmp(A.tags[k].name, name))
            continue;
        A.tags[k].count++;
        return;
    }
    arr_push(A.tags, ((Tag_Count){name, 1}));
}

static void refresh_summary(void)
{
    memset(&A.stats, 0, sizeof A.stats);
    A.stats.total = len_of(A.items);
    arr_clear(A.tags);
    for (int c = 0; c < 3; c++)
        arr_clear(A.col[c]);

    for (int i = 0; i < len_of(A.items); i++) {
        const Item *it = &A.items[i];

        A.stats.prio[it->prio]++;
        if (it->status == ST_TODO)
            A.stats.todo++;
        else if (it->status == ST_WIP)
            A.stats.wip++;
        else
            A.stats.done++;
        arr_push(A.col[it->status], i);
        for (int t = 0; t < len_of(it->tags); t++)
            count_tag(it->tags[t]);
    }
    A.stats.percent = A.stats.total ? A.stats.done * 100 / A.stats.total : 0;
    arr_sort(A.tags, cmp_tag);

    for (int c = 0; c < 3; c++)
        if (A.kb_cur[c] >= len_of(A.col[c]))
            A.kb_cur[c] = len_of(A.col[c]) ? len_of(A.col[c]) - 1 : 0;
}

static void refresh_visible(void)
{
    arr_clear(A.vis);
    for (int i = 0; i < len_of(A.items); i++)
        if (in_section(&A.items[i]) && (!A.filter || matches_query(&A.items[i], A.filter)))
            arr_push(A.vis, i);

    switch (A.sort) {
    case SORT_PRIORITY: arr_sort(A.vis, cmp_prio); break;
    case SORT_STATUS: arr_sort(A.vis, cmp_status); break;
    case SORT_TITLE: arr_sort(A.vis, cmp_title); break;
    default: break;
    }
}

void app_refresh(void)
{
    refresh_summary();
    refresh_visible();
}

static int col_len(void) { return len_of(A.col[A.kb_col]); }

static int table_selected(void)
{
    return A.tbl_sel >= 0 && A.tbl_sel < len_of(A.vis) ? A.vis[A.tbl_sel] : -1;
}

static int kanban_selected(void)
{
    int n = col_len();
    if (!n)
        return -1;
    int i = A.kb_cur[A.kb_col];
    return A.col[A.kb_col][i < n ? i : n - 1];
}

int app_selected(void) { return A.view == V_TABLE ? table_selected() : kanban_selected(); }

int app_position(void)
{
    if (A.view == V_KANBAN)
        return col_len() ? (A.kb_cur[A.kb_col] < col_len() ? A.kb_cur[A.kb_col] : col_len() - 1) + 1 : 0;
    return A.tbl_sel >= 0 && len_of(A.vis) ? A.tbl_sel + 1 : 0;
}

const char *app_chord_label(void)
{
    return A.chord_g ? "g..." : A.chord_d ? "d..." : NULL;
}

static void tbl_clamp(void)
{
    if (!len_of(A.vis))
        A.tbl_sel = -1;
    else {
        int r = A.tbl_sel < 0 ? 0 : A.tbl_sel;
        A.tbl_sel = r < len_of(A.vis) ? r : len_of(A.vis) - 1;
    }
}

static void tbl_next(void)
{
    if (len_of(A.vis))
        A.tbl_sel = A.tbl_sel < 0 || A.tbl_sel >= len_of(A.vis) - 1 ? 0 : A.tbl_sel + 1;
}

static void tbl_prev(void)
{
    if (len_of(A.vis))
        A.tbl_sel = A.tbl_sel <= 0 ? len_of(A.vis) - 1 : A.tbl_sel - 1;
}

static void kb_next(void)
{
    int n = col_len(), *c = &A.kb_cur[A.kb_col];
    if (n)
        *c = *c >= n - 1 ? 0 : *c + 1;
}

static void kb_prev(void)
{
    int n = col_len(), *c = &A.kb_cur[A.kb_col];
    if (n)
        *c = *c <= 0 ? n - 1 : *c - 1;
}

static void commit(const char *status, const char *audit)
{
    int saved = store_save(A.items);

    sync_request_push();
    app_refresh();
    app_set_status("%s", status);
    app_log("%s", audit);
    if (saved)
        app_set_status("Save failed: %s", store_error());
}

static void filter_changed(void)
{
    app_refresh();
    A.tbl_sel = 0;
}

static void set_filter(char *f)
{
    free(A.filter);
    A.filter = f;
    filter_changed();
}

typedef struct {
    int id;
    char *title, *desc;
    char **tags;
    int prio, status;
} Draft;

static void update_task(Draft *d)
{
    Item *it = &A.items[d->id];
    char snip[64], audit[96];

    snippet(snip, sizeof snip, d->title, 28);
    free(it->title);
    free(it->desc);
    tags_free(it->tags);
    it->title = d->title;
    it->desc = d->desc;
    it->tags = d->tags;
    it->prio = (uint8_t)d->prio;
    snprintf(audit, sizeof audit, "Edited \"%s\"", snip);
    commit("Task updated", audit);
}

static void create_task(Draft *d)
{
    char snip[64], audit[96];
    Item it = {d->title, d->desc, d->tags, (uint8_t)d->prio, (uint8_t)d->status};

    snippet(snip, sizeof snip, d->title, 28);
    arr_push(A.items, it);
    snprintf(audit, sizeof audit, "Created \"%s\"", snip);
    commit("Task created", audit);
    A.tbl_sel = len_of(A.vis) ? len_of(A.vis) - 1 : 0;
}

static void save_task(Draft *d)
{
    A.mode = M_NORMAL;
    if (d->id < 0)
        create_task(d);
    else if (d->id < len_of(A.items))
        update_task(d);
    else
        item_free(&(Item){d->title, d->desc, d->tags, 0, 0});
}

static void delete_task(int idx)
{
    if (idx < 0 || idx >= len_of(A.items))
        return;
    if (A.has_deleted)
        item_free(&A.deleted);
    A.deleted = A.items[idx];
    A.deleted_idx = idx;
    A.has_deleted = true;
    arr_remove_at(A.items, (size_t)idx);
    char snip[64], st[128], audit[96];
    snippet(snip, sizeof snip, A.deleted.title, 28);
    snprintf(st, sizeof st, "Deleted \"%s\" (press 'u' to undo)", snip);
    snprintf(audit, sizeof audit, "Deleted \"%s\"", snip);
    commit(st, audit);
    tbl_clamp();
}

static void undo_delete(void)
{
    if (!A.has_deleted) {
        app_set_status("Nothing to undo");
        return;
    }
    int at = A.deleted_idx < len_of(A.items) ? A.deleted_idx : len_of(A.items);
    arr_insert_at(A.items, (size_t)at, A.deleted);
    A.has_deleted = false;
    char snip[64], st[96];
    snippet(snip, sizeof snip, A.items[at].title, 28);
    snprintf(st, sizeof st, "Restored \"%s\"", snip);
    commit(st, st);
    A.tbl_sel = at;
}

static void change_status(int idx, int to, bool shift)
{
    if (idx < 0 || idx >= len_of(A.items))
        return;
    Item *it = &A.items[idx];
    it->status = (uint8_t)to;
    char snip[64], st[96], audit[128];
    snippet(snip, sizeof snip, it->title, 28);
    if (shift) {
        snprintf(st, sizeof st, "Shifted task to %s", status_label(to));
        snprintf(audit, sizeof audit, "Shifted \"%s\" to %s", snip, status_label(to));
    } else {
        snprintf(st, sizeof st, "Status changed to %s", status_label(to));
        snprintf(audit, sizeof audit, "\"%s\" status → %s", snip, status_label(to));
    }
    commit(st, audit);
}

static void shift_status(int idx, bool forward)
{
    if (idx < 0 || idx >= len_of(A.items))
        return;
    int s = A.items[idx].status;
    change_status(idx, forward ? (s + 1) % 3 : (s + 2) % 3, true);
    A.kb_col = forward ? (A.kb_col < 2 ? A.kb_col + 1 : 2) : (A.kb_col > 0 ? A.kb_col - 1 : 0);
}

static void set_priority(int idx, int p)
{
    if (idx < 0 || idx >= len_of(A.items))
        return;
    A.items[idx].prio = (uint8_t)p;
    char snip[64], st[64], audit[128];
    snippet(snip, sizeof snip, A.items[idx].title, 28);
    snprintf(st, sizeof st, "Priority set to %s", prio_label(p));
    snprintf(audit, sizeof audit, "\"%s\" priority → %s", snip, prio_label(p));
    commit(st, audit);
}

static void move_task(int idx, int dir)
{
    int to = idx + dir;
    if (idx < 0 || idx >= len_of(A.items) || to < 0 || to >= len_of(A.items))
        return;
    Item tmp = A.items[idx];
    A.items[idx] = A.items[to];
    A.items[to] = tmp;
    char snip[64], st[32], audit[96];
    snippet(snip, sizeof snip, A.items[to].title, 28);
    snprintf(st, sizeof st, "Moved task %s", dir < 0 ? "up" : "down");
    snprintf(audit, sizeof audit, "Moved \"%s\" %s", snip, dir < 0 ? "up" : "down");
    commit(st, audit);
    if (A.view == V_TABLE)
        dir < 0 ? tbl_prev() : tbl_next();
    else
        dir < 0 ? kb_prev() : kb_next();
}

static void toggle_view(void)
{
    A.view = A.view == V_TABLE ? V_KANBAN : V_TABLE;
    app_set_status("Switched to %s view", A.view == V_KANBAN ? "Kanban Board" : "Table Grid");
}

static void cycle_sort(void)
{
    static const char *const names[] = {"Natural", "Priority", "Status", "Title"};
    A.sort = (Sort)((A.sort + 1) % 4);
    app_refresh();
    app_set_status("Sort: %s", names[A.sort]);
}

static void cycle_tag_filter(void)
{
    if (!len_of(A.tags)) {
        free(A.filter);
        A.filter = NULL;
        app_set_status("No tags available to filter");
        return;
    }
    int next = 0;
    if (A.filter) {
        const char *cur = A.filter[0] == '#' ? A.filter + 1 : A.filter;
        int pos = -1;
        for (int i = 0; i < len_of(A.tags); i++)
            if (!strcmp(A.tags[i].name, cur))
                pos = i;
        next = pos < 0 ? 0 : pos + 1;
    }
    if (next < len_of(A.tags)) {
        size_t n = strlen(A.tags[next].name) + 2;
        char *f = xmalloc(n);
        snprintf(f, n, "#%s", A.tags[next].name);
        app_set_status("Filter: %s", f);
        set_filter(f);
    } else {
        app_set_status("Filter cleared");
        set_filter(NULL);
    }
}

static void clear_filter(void)
{
    bool cleared = false;
    if (A.filter) {
        free(A.filter);
        A.filter = NULL;
        cleared = true;
    }
    if (A.section != NAV_ALL) {
        A.section = NAV_ALL;
        A.nav_idx = 0;
        cleared = true;
    }
    if (cleared) {
        filter_changed();
        app_set_status("Filter reset to All");
    }
}

static void nav_apply(void)
{
    if (A.nav_idx < NAV_COUNT) {
        A.section = (Nav_Section)A.nav_idx;
        free(A.filter);
        A.filter = NULL;
        app_set_status("View: %s", nav_label(A.section));
    } else if (A.nav_idx - NAV_COUNT < len_of(A.tags)) {
        const char *tag = A.tags[A.nav_idx - NAV_COUNT].name;
        A.section = NAV_ALL;
        size_t n = strlen(tag) + 2;
        char *f = xmalloc(n);
        snprintf(f, n, "#%s", tag);
        free(A.filter);
        A.filter = f;
        app_set_status("Filtered by tag #%s", tag);
    } else {
        return;
    }
    filter_changed();
}

static int nav_total(void) 
{ 
    return NAV_COUNT + (len_of(A.tags) < NAV_TAGS_SHOWN ? len_of(A.tags) : NAV_TAGS_SHOWN); 
}

static void reload_from_disk(void)
{
    Item *v = NULL;

    if (store_load(&v))
        return;
    items_free(A.items);
    A.items = v;
    app_refresh();
    tbl_clamp();
}

static void trigger_sync(void)
{
    if (sync_enabled) {
        sync_force();
        app_set_status("Manual sync requested");
        app_log("Sync: manual sync requested");
    } else {
        app_set_status("Sync is disabled (no Git repo configured)");
    }
}

void app_sync_status(Sync_Kind kind, const char *msg)
{
    switch (kind) {
    case SY_UPDATED:
        reload_from_disk();
        app_set_status("Pulled remote changes");
        app_log("Sync: pulled remote changes");
        break;
    case SY_OFFLINE:
        app_set_status("Offline — will sync when connection returns");
        app_log("Sync: offline, will retry");
        break;
    case SY_CONFLICT:
        reload_from_disk();
        app_set_status("Sync conflict — local backup saved to %s", msg);
        app_log("Sync: conflict, backup saved to %s", msg);
        break;
    case SY_ERROR:
        app_set_status("Sync error: %s", msg);
        app_log("Sync: error — %s", msg);
        break;
    default:
        break;
    }
    A.sync = kind;
    snprintf(A.sync_msg, sizeof A.sync_msg, "%s", msg ? msg : "");
}

static void ed_focus(void)
{
    Editor *e = &A.ed;
    String_Builder *b = e->field == FF_TITLE ? &e->title : e->field == FF_DESC ? &e->desc : &e->tags;
    if (e->field == FF_PRIO)
        return;
    ti_focus(&e->in, b);
}

static void ed_open_create(int status)
{
    Editor *e = &A.ed;
    sb_clear(&e->title);
    sb_clear(&e->desc);
    sb_clear(&e->tags);
    e->prio = P_MED;
    e->field = FF_TITLE;
    e->edit_idx = -1;
    e->create_status = status;
    ed_focus();
    A.mode = M_CREATE;
}

static void ed_open_edit(int idx)
{
    if (idx < 0 || idx >= len_of(A.items))
        return;
    Editor *e = &A.ed;
    const Item *it = &A.items[idx];
    sb_set(&e->title, it->title);
    sb_set(&e->desc, it->desc);
    sb_clear(&e->tags);
    for (int i = 0; i < len_of(it->tags); i++) {
        if (i)
            sb_append(&e->tags, ", ", 2);
        sb_append(&e->tags, it->tags[i], strlen(it->tags[i]));
    }
    e->prio = it->prio;
    e->field = FF_TITLE;
    e->edit_idx = idx;
    ed_focus();
    A.mode = M_EDIT;
}

static char *trimmed(const char *s, size_t n)
{
    while (n && (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r')) {
        s++;
        n--;
    }
    while (n && (s[n - 1] == ' ' || s[n - 1] == '\t' || s[n - 1] == '\n' || s[n - 1] == '\r'))
        n--;
    return xstrndup(s, n);
}

static void ed_save(void)
{
    Editor *e = &A.ed;
    Draft d = {.id = e->edit_idx, .prio = e->prio, .status = e->create_status};
    d.title = trimmed(sb_cstr(&e->title), e->title.count);
    if (!d.title[0]) {
        free(d.title);
        return;
    }
    d.desc = trimmed(sb_cstr(&e->desc), e->desc.count);
    const char *p = sb_cstr(&e->tags);
    while (*p) {
        const char *comma = strchr(p, ',');
        size_t n = comma ? (size_t)(comma - p) : strlen(p);
        char *t = trimmed(p, n);
        if (*t) {
            arr_push(d.tags, t);
        } else {
            free(t);
        }
        p += n + (comma ? 1 : 0);
    }
    save_task(&d);
}

static void ed_key(Key k)
{
    Editor *e = &A.ed;
    if (k.ctrl && k.c == 'c') {
        A.mode = M_NORMAL;
        return;
    }
    String_Builder *b = e->field == FF_TITLE ? &e->title : e->field == FF_DESC ? &e->desc : &e->tags;
    if (k.c == HK_ESC) {
        if (e->field == FF_PRIO || !ti_esc(&e->in, b))
            A.mode = M_NORMAL;
        return;
    }
    if ((k.c == HK_ENTER && e->field != FF_DESC) || (k.ctrl && k.c == 's')) {
        ed_save();
        return;
    }
    if ((k.ctrl && (k.c == 'j' || k.c == 'n')) || k.c == HK_TAB || (k.c == HK_DOWN && e->field != FF_DESC)) {
        e->field = (Form_Field)((e->field + 1) & 3);
        ed_focus();
        return;
    }
    if ((k.ctrl && (k.c == 'k' || k.c == 'p')) || k.c == HK_BACKTAB || (k.c == HK_UP && e->field != FF_DESC)) {
        e->field = (Form_Field)((e->field + 3) & 3);
        ed_focus();
        return;
    }
    if (e->field != FF_PRIO) {
        ti_key(&e->in, b, k, e->field == FF_DESC);
        return;
    }
    if (k.ctrl)
        return;
    switch (k.c) {
    case ' ': case 'l': case 'p': case HK_RIGHT:
        e->prio = prio_next(e->prio);
        break;
    case 'h': case 'P': case HK_LEFT:
        e->prio = prio_prev(e->prio);
        break;
    case '1': case '2': case '3': case '4':
        e->prio = (int)k.c - '1';
        break;
    }
}

static void run_command(const char *cmd)
{
    while (*cmd == ' ')
        cmd++;
    size_t n = strlen(cmd);
    while (n && cmd[n - 1] == ' ')
        n--;
    char c[64];
    snprintf(c, sizeof c, "%.*s", (int)n, cmd);
    A.mode = M_NORMAL;
    if (!strcmp(c, "sync") || !strcmp(c, "s"))
        trigger_sync();
    else if (!strcmp(c, "q") || !strcmp(c, "quit") || !strcmp(c, "wq"))
        A.quit = true;
    else if (!strcmp(c, "w"))
        app_set_status("Already saved automatically");
    else if (!strcmp(c, "help") || !strcmp(c, "h")) {
        A.help_scroll = 0;
        A.mode = M_HELP;
    } else if (c[0])
        app_set_status("Unknown command: %s", c);
}

static void cmd_open(int find)
{
    Cmd_Line *c = &A.cmd;
    c->find = find;
    sb_set(&c->buf, find && A.filter ? A.filter : "");
    ti_focus(&c->in, &c->buf);
    A.mode = find ? M_FIND : M_CMD;
}

static void submit_find(void)
{
    char *q = trimmed(sb_cstr(&A.cmd.buf), A.cmd.buf.count);

    A.mode = M_NORMAL;
    if (*q) {
        set_filter(q);
        return;
    }
    free(q);
    set_filter(NULL);
}

static void cmd_key(Key k)
{
    Cmd_Line *c = &A.cmd;

    if (k.c == HK_ENTER && c->find) {
        submit_find();
    } else if (k.c == HK_ENTER) {
        run_command(sb_cstr(&c->buf));
    } else if (k.c == HK_ESC) {
        if (!ti_esc(&c->in, &c->buf))
            A.mode = M_NORMAL;
    } else if (k.ctrl && k.c == 'c') {
        A.mode = M_NORMAL;
    } else {
        ti_key(&c->in, &c->buf, k, false);
    }
}

static void help_key(Key k)
{
    if (k.ctrl)
        return;
    switch (k.c) {
    case HK_ESC: case '?': case 'q': case HK_ENTER: case ' ':
        A.help_scroll = 0;
        A.mode = M_NORMAL;
        break;
    case 'j': case HK_DOWN:
        if (A.help_scroll < A.help_max)
            A.help_scroll++;
        break;
    case 'k': case HK_UP:
        if (A.help_scroll > 0)
            A.help_scroll--;
        break;
    }
}

static bool task_command(Key k, int sel)
{
    if (k.ctrl)
        return false;
    switch (k.c) {
    case 'e': case 'c': case HK_ENTER:
        if (sel >= 0)
            ed_open_edit(sel);
        return true;
    case 'x':
        delete_task(sel);
        return true;
    case 'J':
        move_task(sel, 1);
        return true;
    case 'K':
        move_task(sel, -1);
        return true;
    case 'p':
        if (sel >= 0)
            set_priority(sel, prio_next(A.items[sel].prio));
        return true;
    case 'P':
        if (sel >= 0)
            set_priority(sel, prio_prev(A.items[sel].prio));
        return true;
    case '1': case '2': case '3': case '4':
        set_priority(sel, (int)k.c - '1');
        return true;
    }
    return false;
}

static bool press_g(void)
{
    A.chord_d = false;
    bool done = A.chord_g;
    A.chord_g = !A.chord_g;
    return done;
}

static bool press_d(void)
{
    A.chord_g = false;
    bool done = A.chord_d;
    A.chord_d = !A.chord_d;
    return done;
}

static bool table_key(Key k)
{
    int sel = table_selected(), n = len_of(A.vis);
    if (task_command(k, sel))
        return true;
    if (k.ctrl) {
        if (k.c == 'd' && n)
            A.tbl_sel = A.tbl_sel < 0 ? 0 : A.tbl_sel + PAGE < n ? A.tbl_sel + PAGE : n - 1;
        else if (k.c == 'u' && n)
            A.tbl_sel = A.tbl_sel < 0 ? 0 : A.tbl_sel > PAGE ? A.tbl_sel - PAGE : 0;
        else
            return false;
        return true;
    }
    switch (k.c) {
    case HK_PGDN:
        if (n)
            A.tbl_sel = A.tbl_sel < 0 ? 0 : A.tbl_sel + PAGE < n ? A.tbl_sel + PAGE : n - 1;
        break;
    case HK_PGUP:
        if (n)
            A.tbl_sel = A.tbl_sel < 0 ? 0 : A.tbl_sel > PAGE ? A.tbl_sel - PAGE : 0;
        break;
    case 'j': case HK_DOWN: tbl_next(); break;
    case 'k': case HK_UP: tbl_prev(); break;
    case 'G': case HK_END: case 'L':
        if (n)
            A.tbl_sel = n - 1;
        break;
    case 'H': case HK_HOME:
        if (n)
            A.tbl_sel = 0;
        break;
    case 'g':
        if (press_g() && n)
            A.tbl_sel = 0;
        break;
    case 'd':
        if (press_d())
            delete_task(sel);
        break;
    case 'a': case 'o': case 'i': ed_open_create(ST_TODO); break;
    case ' ':
        if (sel >= 0)
            change_status(sel, (A.items[sel].status + 1) % 3, false);
        break;
    default:
        return false;
    }
    return true;
}

static bool kanban_key(Key k)
{
    int sel = kanban_selected(), n = col_len();
    if (task_command(k, sel))
        return true;
    if (k.ctrl)
        return false;
    switch (k.c) {
    case 'j': case HK_DOWN: kb_next(); break;
    case 'k': case HK_UP: kb_prev(); break;
    case 'h': case HK_LEFT: A.kb_col = A.kb_col > 0 ? A.kb_col - 1 : 2; break;
    case 'l': case HK_RIGHT: A.kb_col = A.kb_col < 2 ? A.kb_col + 1 : 0; break;
    case 'G': case HK_END:
        if (n)
            A.kb_cur[A.kb_col] = n - 1;
        break;
    case HK_HOME: A.kb_cur[A.kb_col] = 0; break;
    case 'g':
        if (press_g())
            A.kb_cur[A.kb_col] = 0;
        break;
    case 'd':
        if (press_d())
            delete_task(sel);
        break;
    case 'H': shift_status(sel, false); break;
    case 'L': case ' ': shift_status(sel, true); break;
    case 'a': case 'o': case 'i': ed_open_create(A.kb_col); break;
    default:
        return false;
    }
    return true;
}

static bool nav_key(Key k)
{
    if (k.ctrl)
        return false;
    int total = nav_total();
    switch (k.c) {
    case 'j': case HK_DOWN:
        A.nav_idx = (A.nav_idx + 1) % total;
        nav_apply();
        return true;
    case 'k': case HK_UP:
        A.nav_idx = A.nav_idx == 0 ? total - 1 : A.nav_idx - 1;
        nav_apply();
        return true;
    case ' ':
        nav_apply();
        return true;
    case HK_ENTER:
        nav_apply();
        A.focus = F_MAIN;
        return true;
    case '1': case '2': case '3': case '4': case '5':
        A.nav_idx = (int)k.c - '1';
        nav_apply();
        return true;
    }
    return false;
}

static bool inspector_key(Key k)
{
    if (k.ctrl)
        return false;
    int max = A.insp_total > A.insp_vis ? A.insp_total - A.insp_vis : 0;
    if (k.c == 'j' || k.c == HK_DOWN)
        A.insp_scroll = A.insp_scroll < max ? A.insp_scroll + 1 : max;
    else if (k.c == 'k' || k.c == HK_UP)
        A.insp_scroll = A.insp_scroll > 0 ? A.insp_scroll - 1 : 0;
    else
        return false;
    return true;
}

static void global_key(Key k)
{
    if (k.ctrl)
        return;
    switch (k.c) {
    case 'v': toggle_view(); break;
    case 's': cycle_sort(); break;
    case 'h': case HK_LEFT: A.focus = (Focus)((A.focus + 2) % 3); break;
    case 'l': case HK_RIGHT: A.focus = (Focus)((A.focus + 1) % 3); break;
    case 'u': undo_delete(); break;
    case '/': cmd_open(1); break;
    case ':': cmd_open(0); break;
    case 't': cycle_tag_filter(); break;
    case HK_ESC: clear_filter(); break;
    case '?':
        A.help_scroll = 0;
        A.mode = M_HELP;
        break;
    case 'S': case HK_F5: trigger_sync(); break;
    }
}

static void normal_key(Key k)
{
    if (!((k.c == 'g' || k.c == 'd') && !k.ctrl))
        A.chord_g = A.chord_d = false;

    if (k.ctrl) {
        if (k.c == 'c') {
            A.quit = true;
            return;
        }
        if (k.c == 'w') {
            A.focus = (Focus)((A.focus + 1) % 3);
            return;
        }
    } else if (k.c == 'q') {
        A.quit = true;
        return;
    } else if (k.c == HK_TAB) {
        A.focus = (Focus)((A.focus + 1) % 3);
        return;
    } else if (k.c == HK_BACKTAB) {
        A.focus = (Focus)((A.focus + 2) % 3);
        return;
    }

    if (A.focus == F_NAV && nav_key(k))
        return;
    if (A.focus == F_INSPECTOR && inspector_key(k))
        return;
    if (A.view == V_TABLE ? table_key(k) : kanban_key(k))
        return;
    global_key(k);
}

void app_key(Key k)
{
    switch (A.mode) {
    case M_NORMAL: normal_key(k); break;
    case M_CREATE:
    case M_EDIT: ed_key(k); break;
    case M_FIND:
    case M_CMD: cmd_key(k); break;
    case M_HELP: help_key(k); break;
    }
}

void app_init(const Config *cfg)
{
    (void)cfg;
    memset(&A, 0, sizeof A);
    A.focus = F_MAIN;
    A.sync = SY_DISABLED;
    A.tbl_sel = -1;
    ti_init(&A.ed.in);
    ti_init(&A.cmd.in);
    A.ed.edit_idx = -1;
    A.ed.prio = P_MED;
    if (store_load(&A.items))
        app_set_status("%s", store_error());
    app_refresh();
    if (len_of(A.items))
        A.tbl_sel = 0;
    A.sync = sync_enabled ? SY_IDLE : SY_DISABLED;
}

void app_free(void)
{
    items_free(A.items);
    if (A.has_deleted)
        item_free(&A.deleted);
    free(A.filter);
    arr_free(A.vis);
    for (int c = 0; c < 3; c++)
        arr_free(A.col[c]);
    arr_free(A.tags);
    sb_free(&A.ed.title);
    sb_free(&A.ed.desc);
    sb_free(&A.ed.tags);
    sb_free(&A.cmd.buf);
    free(A.ed.in.clip);
    free(A.cmd.in.clip);
}

void app_shutdown(void) { sync_shutdown(); }
