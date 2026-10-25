#include "hopes.h"

char *default_data_dir(void)
{
    char *base = plat_base_dir(0);
    size_t n = strlen(base) + 8;
    char *p = xmalloc(n);

    snprintf(p, n, "%s/hopes", base);
    free(base);
    return p;
}

static char *config_path(void)
{
    const char *env = getenv("HOPES_CONFIG");
    if (env && *env)
        return xstrdup(env);
    char *b = plat_base_dir(1);
    size_t n = strlen(b) + 24;
    char *p = xmalloc(n);
    snprintf(p, n, "%s/hopes/config.toml", b);
    free(b);
    return p;
}

static char *skip_ws(char *p)
{
    while (*p == ' ' || *p == '\t')
        p++;
    return p;
}

static char *parse_str(char **pp)
{
    char *p = *pp, q = *p++;
    String_Builder b = {0};
    sb_reserve(&b, 0);
    while (*p && *p != q) {
        if (q == '"' && *p == '\\' && p[1]) {
            p++;
            char c = *p++;
            sb_putc(&b, c == 'n' ? '\n' : c == 't' ? '\t' : c);
        } else {
            sb_putc(&b, *p++);
        }
    }
    if (*p != q) {
        sb_free(&b);
        return NULL;
    }
    *pp = p + 1;
    return b.data;
}

static void set_str(char **dst, char *v)
{
    free(*dst);
    *dst = v;
}

static void parse_array(Config *c, char *p)
{
    tags_free(c->command);
    c->command = NULL;
    p++;
    for (;;) {
        char *s;

        p = skip_ws(p);
        if (*p == ',') {
            p++;
            continue;
        }
        if (*p != '"' && *p != '\'')
            return;
        s = parse_str(&p);
        if (!s)
            return;
        arr_push(c->command, s);
    }
}

static void assign(Config *c, const char *section, const char *key, char *val)
{
    int is_sync = !strcmp(section, "sync"), is_store = !strcmp(section, "storage");
    int isstr = *val == '"' || *val == '\'';
    char *sv = isstr ? parse_str(&val) : NULL;
    int b = !strncmp(val, "true", 4);

    if (is_sync) {
        if (!strcmp(key, "enabled"))
            c->sync_enabled = b;
        else if (!strcmp(key, "pull_on_startup"))
            c->pull_on_startup = b;
        else if (!strcmp(key, "debounce_seconds") && isdigit((unsigned char)*val))
            c->debounce = atoi(val);
        else if (!strcmp(key, "data_dir") && sv)
            set_str(&c->sync_dir, sv), sv = NULL;
        else if (!strcmp(key, "remote") && sv)
            set_str(&c->remote, sv), sv = NULL;
        else if (!strcmp(key, "branch") && sv)
            set_str(&c->branch, sv), sv = NULL;
    } else if (is_store) {
        if (!strcmp(key, "backend") && sv) {
            c->backend = !strcmp(sv, "sqlite") ? BE_SQLITE : !strcmp(sv, "exec") ? BE_EXEC : BE_JSON;
        } else if (!strcmp(key, "dir") && sv) {
            set_str(&c->dir, sv), sv = NULL;
        } else if (!strcmp(key, "command") && *val == '[') {
            parse_array(c, val);
        }
    }
    free(sv);
}

static char *join_array(char *val, char **next)
{
    String_Builder joined = {0};

    sb_set(&joined, val);
    while (*next && **next) {
        char *nl = strchr(*next, '\n');
        int done;

        if (nl)
            *nl = 0;
        sb_append(&joined, *next, strlen(*next));
        done = strchr(*next, ']') != NULL;
        *next = nl ? nl + 1 : NULL;
        if (done)
            break;
    }
    return joined.data;
}

static void parse_line(Config *c, char *section, char *p, char **next)
{
    char *eq = strchr(p, '='), *val, *joined = NULL;

    if (!eq)
        return;
    *eq = 0;
    for (char *kend = eq; kend > p && isspace((unsigned char)kend[-1]);)
        *--kend = 0;
    val = skip_ws(eq + 1);
    if (*val == '[' && !strchr(val, ']') && *next)
        val = joined = join_array(val, next);
    assign(c, section, p, val);
    free(joined);
}

static void parse_toml(Config *c, char *text)
{
    char section[32] = "";

    for (char *line = text, *next; line && *line; line = next) {
        char *p = skip_ws(line), *end;

        next = strchr(line, '\n');
        if (next)
            *next++ = 0;
        if (*p == '#' || !*p)
            continue;
        end = *p == '[' ? strchr(p, ']') : NULL;
        if (end)
            *end = 0;
        if (*p != '[')
            parse_line(c, section, p, &next);
        else if (end)
            snprintf(section, sizeof section, "%s", p + 1);
    }
}

void config_load(Config *c)
{
    memset(c, 0, sizeof *c);
    c->sync_enabled = 1;
    c->debounce = 5;
    c->remote = xstrdup("origin");
    c->branch = xstrdup("main");
    c->pull_on_startup = 1;

    char *path = config_path();
    FILE *f = fopen(path, "r");
    free(path);
    if (!f)
        return;
    String_Builder b = {0};
    char chunk[1024];
    size_t n;
    while ((n = fread(chunk, 1, sizeof chunk, f)) > 0)
        sb_append(&b, chunk, n);
    fclose(f);
    text_normalize(&b);
    if (b.data)
        parse_toml(c, b.data);
    sb_free(&b);
}
