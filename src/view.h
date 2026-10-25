#include "hopes.h"

static const char *const VIEW_MODES[] = {"Table", "Kanban"};
static const char *const SORT_MODES[] = {"Natural", "Priority", "Status", "Due", "Title"};
static const char *const SECTIONS[] = {"All", "Todo", "InProgress", "Complete", "Urgent", "Overdue"};

typedef struct {
    int view, sort, section, seen;
    char *filter;
} Saved_View;

enum { SV_VIEW = 1, SV_SORT = 2, SV_SECTION = 4, SV_ALL = 7 };

static char *view_dir(void)
{
    char *state = plat_state_dir();
    size_t n = strlen(state) + 8;
    char *dir = xmalloc(n);

    snprintf(dir, n, "%s/hopes", state);
    free(state);
    return dir;
}

static char *view_path(const char *dir, const char *name)
{
    size_t n = strlen(dir) + strlen(name) + 2;
    char *path = xmalloc(n);

    snprintf(path, n, "%s/%s", dir, name);
    return path;
}

void view_encode(String_Builder *out)
{
    sb_append(out, "{\"view_mode\":", 13);
    put_str(out, VIEW_MODES[A.view]);
    sb_append(out, ",\"sort_mode\":", 13);
    put_str(out, SORT_MODES[A.sort]);
    sb_append(out, ",\"nav_section\":", 15);
    put_str(out, SECTIONS[A.section]);
    sb_append(out, ",\"tag_filter\":", 14);
    if (A.filter)
        put_str(out, A.filter);
    else
        sb_append(out, "null", 4);
    sb_putc(out, '}');
}

static int saved_field(Json_Parser *j, const char *key, Saved_View *v)
{
    if (!strcmp(key, "view_mode")) {
        v->seen |= SV_VIEW;
        return (v->view = parse_enum(j, VIEW_MODES, 2)) < 0 ? -1 : 0;
    }
    if (!strcmp(key, "sort_mode")) {
        v->seen |= SV_SORT;
        return (v->sort = parse_enum(j, SORT_MODES, SORT_COUNT)) < 0 ? -1 : 0;
    }
    if (!strcmp(key, "nav_section")) {
        v->seen |= SV_SECTION;
        return (v->section = parse_enum(j, SECTIONS, NAV_COUNT)) < 0 ? -1 : 0;
    }
    if (!strcmp(key, "tag_filter")) {
        free(v->filter);
        return parse_nullable(j, &v->filter);
    }
    return skip_value(j, 0);
}

static int saved_parse(const char *data, size_t len, Saved_View *v)
{
    Json_Parser j = {data, data + len};
    int more = list_open(&j, '{', '}');

    while (more > 0) {
        char *key;
        int rc;

        if (parse_key(&j, &key))
            return -1;
        rc = saved_field(&j, key, v);
        free(key);
        if (rc)
            return -1;
        more = list_next(&j, '}');
    }
    ws(&j);
    return more < 0 || j.p != j.end || v->seen != SV_ALL ? -1 : 0;
}

static int tag_exists(const char *name)
{
    for (int i = 0; i < len_of(A.tags); i++)
        if (!strcmp(A.tags[i].name, name))
            return 1;
    return 0;
}

static void restore_nav(void)
{
    int shown = len_of(A.tags) < NAV_TAGS_SHOWN ? len_of(A.tags) : NAV_TAGS_SHOWN;

    A.nav_idx = A.section;
    if (!A.filter || A.filter[0] != '#')
        return;
    for (int i = 0; i < shown; i++)
        if (!strcmp(A.tags[i].name, A.filter + 1))
            A.nav_idx = NAV_COUNT + i;
}

static void saved_apply(Saved_View *v)
{
    A.view = (View_Mode)v->view;
    A.sort = (Sort)v->sort;
    A.section = (Nav_Section)v->section;
    free(A.filter);
    A.filter = NULL;
    if (v->filter && (v->filter[0] != '#' || tag_exists(v->filter + 1))) {
        A.filter = v->filter;
        v->filter = NULL;
    }
    app_refresh();
    restore_nav();
    tbl_clamp();
}

int view_decode(const char *data, size_t len)
{
    Saved_View v = {0};
    int rc = saved_parse(data, len, &v);

    if (!rc)
        saved_apply(&v);
    free(v.filter);
    return rc;
}

void view_save(void)
{
    String_Builder b = {0};
    char *dir = view_dir(), *path = view_path(dir, "view.json"), *tmp = view_path(dir, "view.tmp");
    FILE *f;

    mkdirs(dir);
    view_encode(&b);
    f = fopen(tmp, "wb");
    if (f) {
        fwrite(b.data, 1, b.count, f);
        if (fclose(f) || plat_replace(tmp, path))
            remove(tmp);
    }
    sb_free(&b);
    free(dir);
    free(path);
    free(tmp);
}

void view_load(void)
{
    char *dir = view_dir(), *path = view_path(dir, "view.json");
    String_Builder b = {0};
    char chunk[512];
    size_t n;
    FILE *f = fopen(path, "rb");

    if (f) {
        while ((n = fread(chunk, 1, sizeof chunk, f)) > 0)
            sb_append(&b, chunk, n);
        fclose(f);
        if (b.data)
            view_decode(b.data, b.count);
    }
    sb_free(&b);
    free(dir);
    free(path);
}
