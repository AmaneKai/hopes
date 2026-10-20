#include "hopes.h"

#ifdef HOPES_SQLITE
#include <sqlite3.h>
static sqlite3 *db;
#endif

static int kind;
static char *dir, *path, *tmp_path;
static char **cmd;
static char err_msg[1400];

const char *store_dir(void)
{
    return dir;
}

const char *store_error(void)
{
    return err_msg;
}

static int fail(const char *what)
{
    snprintf(err_msg, sizeof err_msg, "%s: %s", what, strerror(errno));
    return -1;
}

static void mkdirs(const char *p)
{
    char *s = xstrdup(p);

    for (char *q = s + 1; *q; q++) {
        if (*q != '/')
            continue;
        *q = 0;
        plat_mkdir(s);
        *q = '/';
    }
    plat_mkdir(s);
    free(s);
}

static int open_sqlite(void)
{
#ifdef HOPES_SQLITE
    char db_path[1100];
    const char *schema =
        "CREATE TABLE IF NOT EXISTS items (position INTEGER PRIMARY KEY, data TEXT NOT NULL)";

    snprintf(db_path, sizeof db_path, "%s/hopes.db", dir);
    if (sqlite3_open(db_path, &db) == SQLITE_OK && sqlite3_exec(db, schema, 0, 0, 0) == SQLITE_OK)
        return 0;
    snprintf(err_msg, sizeof err_msg, "sqlite: %s", db ? sqlite3_errmsg(db) : "open failed");
#else
    snprintf(err_msg, sizeof err_msg, "sqlite backend not compiled in; rebuild with `make SQLITE=1`");
#endif
    return -1;
}

int store_open(const Config *c)
{
    size_t n;

    free(dir);
    free(path);
    free(tmp_path);
    kind = c->backend;
    dir = c->dir ? xstrdup(c->dir) : c->sync_dir ? xstrdup(c->sync_dir) : default_data_dir();
    mkdirs(dir);
    n = strlen(dir) + 24;
    path = xmalloc(n);
    tmp_path = xmalloc(n);
    snprintf(path, n, "%s/items.json", dir);
    snprintf(tmp_path, n, "%s/items.tmp", dir);

    if (kind == BE_SQLITE)
        return open_sqlite();
    if (kind != BE_EXEC)
        return 0;
    if (!len_of(c->command)) {
        snprintf(err_msg, sizeof err_msg, "storage.command is required for the exec backend");
        return -1;
    }
    cmd = c->command;
    return 0;
}

static int read_all(int fd, String_Builder *b)
{
    char chunk[4096];

    for (;;) {
        ssize_t n = read(fd, chunk, sizeof chunk);

        if (n < 0 && errno == EINTR)
            continue;
        if (n <= 0)
            return n < 0 ? -1 : 0;
        sb_append(b, chunk, (size_t)n);
    }
}

static int write_all(int fd, const char *p, size_t n)
{
    while (n) {
        ssize_t w = write(fd, p, n);

        if (w < 0 && errno == EINTR)
            continue;
        if (w <= 0)
            return -1;
        p += w;
        n -= (size_t)w;
    }
    return 0;
}

static int parse_items(const String_Builder *b, Item **v)
{
    size_t i = 0;

    while (i < b->count && strchr(" \n\t\r", b->data[i]))
        i++;
    *v = NULL;
    if (i == b->count)
        return 0;
    if (json_load(b->data, b->count, v)) {
        snprintf(err_msg, sizeof err_msg, "invalid item data");
        return -1;
    }
    return 0;
}

static int run_exec(const char *verb, const String_Builder *in, String_Builder *out)
{
    String_Builder err = {0};
    char **argv = NULL;
    int rc;

    for (int i = 0; i < len_of(cmd); i++)
        arr_push(argv, cmd[i]);
    arr_push(argv, (char *)verb);
    arr_push(argv, NULL);
    rc = plat_run(argv, NULL, in, out, &err, 160, -1);
    if (rc) {
        while (err.count && strchr("\r\n ", err.data[err.count - 1]))
            err.data[--err.count] = 0;
        snprintf(err_msg, sizeof err_msg, "storage command `%s` failed: %s", verb, sb_cstr(&err));
    }
    arr_free(argv);
    sb_free(&err);
    return rc ? -1 : 0;
}

static int load_exec(Item **v)
{
    String_Builder b = {0};
    int rc = run_exec("load", NULL, &b) ? -1 : parse_items(&b, v);

    sb_free(&b);
    return rc;
}

static int save_exec(const Item *v)
{
    String_Builder b = {0};
    int rc;

    json_dump(&b, v, len_of(v), false);
    rc = run_exec("save", &b, NULL);
    sb_free(&b);
    return rc;
}

#ifdef HOPES_SQLITE
static int load_sqlite(Item **v)
{
    String_Builder b = {0};
    sqlite3_stmt *st;
    int rc;

    if (sqlite3_prepare_v2(db, "SELECT data FROM items ORDER BY position", -1, &st, 0) != SQLITE_OK) {
        snprintf(err_msg, sizeof err_msg, "sqlite: %s", sqlite3_errmsg(db));
        return -1;
    }
    sb_putc(&b, '[');
    while (sqlite3_step(st) == SQLITE_ROW) {
        if (b.count > 1)
            sb_putc(&b, ',');
        sb_append(&b, (const char *)sqlite3_column_text(st, 0), (size_t)sqlite3_column_bytes(st, 0));
    }
    sqlite3_finalize(st);
    sb_putc(&b, ']');
    rc = parse_items(&b, v);
    sb_free(&b);
    return rc;
}

static int save_sqlite(const Item *v)
{
    String_Builder b = {0};
    sqlite3_stmt *st;
    int rc;

    sqlite3_exec(db, "BEGIN", 0, 0, 0);
    sqlite3_exec(db, "DELETE FROM items", 0, 0, 0);
    sqlite3_prepare_v2(db, "INSERT INTO items (position, data) VALUES (?1, ?2)", -1, &st, 0);
    for (int i = 0; i < len_of(v); i++) {
        sb_clear(&b);
        json_dump(&b, &v[i], 1, false);

        sqlite3_bind_int(st, 1, i);
        sqlite3_bind_text(st, 2, b.data + 1, (int)b.count - 2, SQLITE_TRANSIENT);
        sqlite3_step(st);
        sqlite3_reset(st);
    }
    sqlite3_finalize(st);
    rc = sqlite3_exec(db, "COMMIT", 0, 0, 0) == SQLITE_OK ? 0 : -1;
    if (rc)
        snprintf(err_msg, sizeof err_msg, "sqlite: %s", sqlite3_errmsg(db));
    sb_free(&b);
    return rc;
}
#endif

static int load_json(Item **v)
{
    String_Builder b = {0};
    char bad[1200];
    int fd = open(path, O_RDONLY | O_BINARY), rc;

    *v = NULL;
    if (fd < 0)
        return errno == ENOENT ? 0 : fail(path);
    rc = read_all(fd, &b);
    close(fd);
    if (rc) {
        sb_free(&b);
        return fail(path);
    }
    rc = parse_items(&b, v);
    sb_free(&b);
    if (!rc)
        return 0;

    snprintf(bad, sizeof bad, "%s.corrupt-%ld", path, (long)time(NULL));
    if (plat_replace(path, bad) == 0)
        snprintf(err_msg, sizeof err_msg, "items.json unreadable; moved to %s", bad);
    return rc;
}

static int save_json(const Item *v)
{
    String_Builder b = {0};
    int fd, rc;

    json_dump(&b, v, len_of(v), true);
    fd = open(tmp_path, O_WRONLY | O_CREAT | O_TRUNC | O_BINARY, 0644);
    if (fd < 0) {
        sb_free(&b);
        return fail(tmp_path);
    }
    rc = write_all(fd, b.data, b.count) || plat_fsync(fd);
    if (close(fd) || rc || plat_replace(tmp_path, path))
        rc = fail("save");
    sb_free(&b);
    return rc;
}

int store_load(Item **v)
{
    *v = NULL;
    if (kind == BE_EXEC)
        return load_exec(v);
#ifdef HOPES_SQLITE
    if (kind == BE_SQLITE)
        return load_sqlite(v);
#endif
    return load_json(v);
}

int store_save(const Item *v)
{
    if (kind == BE_EXEC)
        return save_exec(v);
#ifdef HOPES_SQLITE
    if (kind == BE_SQLITE)
        return save_sqlite(v);
#endif
    return save_json(v);
}
