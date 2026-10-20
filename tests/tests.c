#define HOPES_IMPLEMENTATION
#include "hopes.h"

static int checks, failures;

#define CHECK(cond)                                                         \
    do {                                                                    \
        checks++;                                                           \
        if (!(cond)) {                                                      \
            failures++;                                                     \
            fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
        }                                                                   \
    } while (0)

#define CHECK_STR(got, want) CHECK(!strcmp((got), (want)))

static String_Builder mk(const char *s)
{
    String_Builder b = {0};

    sb_set(&b, s);
    return b;
}

static void test_motions(void)
{
    String_Builder s = mk("abc\ndef");

    CHECK(te_move_left(&s, 0) == 0);
    CHECK(te_move_right(&s, 2) == 2);
    CHECK(te_move_right(&s, 3) == 2);
    sb_free(&s);

    s = mk("helloworld");
    CHECK(te_caret_left(&s, 10) == 9);
    CHECK(te_caret_right(&s, 10) == 10);
    sb_free(&s);

    s = mk("ab\ncd");
    CHECK(te_caret_down(&s, 2) == 5);
    CHECK(te_caret_up(&s, 5) == 2);
    sb_free(&s);

    s = mk("hello world\nsecond");
    CHECK(te_line_start(&s, 8) == 0);
    CHECK(te_line_end(&s, 2) == 10);
    CHECK(te_line_start(&s, 14) == 12);
    CHECK(te_line_end(&s, 14) == 17);
    sb_free(&s);

    s = mk("foo   bar baz");
    CHECK(te_word_forward(&s, 0) == 6);
    CHECK(te_word_forward(&s, 6) == 10);
    CHECK(te_word_end(&s, 0) == 2);
    CHECK(te_word_end(&s, 2) == 8);
    CHECK(te_word_back(&s, 10) == 6);
    CHECK(te_word_back(&s, 6) == 0);
    sb_free(&s);

    s = mk("abcdef\nxy\nqrstuv");
    size_t down = te_move_down(&s, 4), down2 = te_move_down(&s, down);
    CHECK(down == 8);
    CHECK(down2 == 11);
    CHECK(te_move_up(&s, down2) == 8);
    sb_free(&s);
}

static void test_edits(void)
{
    String_Builder s = mk("abc");

    CHECK(te_delete_char(&s, 2) == 1);
    CHECK_STR(s.data, "ab");
    sb_free(&s);

    s = mk("one\ntwo\nthree");
    CHECK(te_delete_line(&s, 5) == 4);
    CHECK_STR(s.data, "one\nthree");
    sb_free(&s);

    s = mk("solo");
    CHECK(te_delete_line(&s, 2) == 0);
    CHECK_STR(s.data, "");
    sb_free(&s);

    s = mk("keep this\nand this");
    CHECK(te_delete_to_line_end(&s, 4) == 3);
    CHECK_STR(s.data, "keep\nand this");
    sb_free(&s);

    s = mk("helloworld");
    size_t c = te_insert(&s, 5, " ", 1);
    CHECK(c == 6);
    CHECK_STR(s.data, "hello world");
    CHECK(te_backspace(&s, c) == 5);
    CHECK_STR(s.data, "helloworld");
    sb_free(&s);

    s = mk("title");
    CHECK(te_open_below(&s, 0, false) == 5);
    CHECK_STR(s.data, "title");
    sb_free(&s);

    s = mk("one");
    CHECK(te_open_below(&s, 1, true) == 4);
    CHECK_STR(s.data, "one\n");
    sb_free(&s);

    s = mk("hey\nyou");
    CHECK(te_clamp_normal(&s, 5) == 5);
    CHECK(te_insert_after(&s, 1) == 2);
    CHECK(te_line_start(&s, 5) == 4);
    CHECK(te_insert_line_end(&s, 5) == 7);
    sb_free(&s);
}

static void test_empty_buffer(void)
{
    String_Builder s = {0};

    CHECK(te_clamp_normal(&s, 0) == 0);
    CHECK(te_move_left(&s, 0) == 0);
    CHECK(te_move_right(&s, 0) == 0);
    CHECK(te_word_forward(&s, 0) == 0);
    CHECK(te_line_end(&s, 0) == 0);
    CHECK(te_delete_char(&s, 0) == 0);
    CHECK(te_delete_line(&s, 0) == 0);
    sb_free(&s);
}

static void test_yank_paste(void)
{
    String_Builder s = mk("hello world");
    char *clip = te_yank(&s, 6, 10);

    CHECK_STR(clip, "world");
    CHECK(te_paste_after(&s, 4, clip) == 9);
    CHECK_STR(s.data, "helloworld world");
    free(clip);
    sb_free(&s);

    s = mk("hello world");
    CHECK(te_delete_range(&s, 0, 4) == 0);
    CHECK_STR(s.data, " world");
    sb_free(&s);
}

static void test_utf8(void)
{
    String_Builder s = mk("雨音 カイ");

    CHECK(te_move_right(&s, 0) == 3);
    CHECK(te_word_forward(&s, 0) == 7);
    CHECK(te_clamp_normal(&s, 100) == 10);
    CHECK(te_backspace(&s, 3) == 0);
    CHECK_STR(s.data, "音 カイ");
    CHECK(te_delete_char(&s, 0) == 0);
    CHECK_STR(s.data, " カイ");
    sb_free(&s);
}

static void test_text_input_modes(void)
{
    Text_Input t;
    String_Builder b = mk("hello world");

    ti_init(&t);
    ti_focus(&t, &b);
    ti_esc(&t, &b);
    CHECK(!t.insert);
    CHECK(t.cursor == 10);
    ti_key(&t, &b, (Key){'0', false}, false);
    ti_key(&t, &b, (Key){'v', false}, false);
    ti_key(&t, &b, (Key){'e', false}, false);
    ti_key(&t, &b, (Key){'y', false}, false);
    CHECK_STR(t.clip, "hello");
    ti_key(&t, &b, (Key){'$', false}, false);
    ti_key(&t, &b, (Key){'p', false}, false);
    CHECK_STR(b.data, "hello worldhello");
    ti_key(&t, &b, (Key){'d', false}, false);
    ti_key(&t, &b, (Key){'d', false}, false);
    CHECK_STR(b.data, "");
    free(t.clip);
    sb_free(&b);
}

static void test_json_roundtrip(void)
{
    const char *src = "[{\"title\":\"a \\\"q\\\" \\u00e9 \\ud83d\\ude00\",\"description\":\"l1\\nl2\","
                      "\"priority\":\"High\",\"status\":\"InProgress\",\"tags\":[\"x\",\"y\"],\"extra\":{\"k\":[1,2.5,null]}}]";
    Item *v = NULL;
    String_Builder out = {0};

    CHECK(json_load(src, strlen(src), &v) == 0);
    CHECK(len_of(v) == 1);
    CHECK_STR(v[0].title, "a \"q\" é 😀");
    CHECK_STR(v[0].desc, "l1\nl2");
    CHECK(v[0].prio == P_HIGH && v[0].status == ST_WIP && len_of(v[0].tags) == 2);

    json_dump(&out, v, 1, true);
    CHECK(strstr(out.data, "{\n    \"title\": \"a \\\"q\\\" é 😀\",\n    \"description\": \"l1\\nl2\",\n") != NULL);
    CHECK(strstr(out.data, "\"tags\": [\n      \"x\",\n      \"y\"\n    ]\n  }\n]") != NULL);
    items_free(v);
    v = NULL;
    CHECK(json_load(out.data, out.count, &v) == 0 && len_of(v) == 1);
    items_free(v);
    sb_free(&out);
}

static void test_json_edge_cases(void)
{
    Item *v = NULL;
    String_Builder out = {0};

    CHECK(json_load("[]", 2, &v) == 0 && len_of(v) == 0);
    json_dump(&out, v, 0, true);
    CHECK_STR(out.data, "[]");
    CHECK(json_load("[{\"title\":\"x\"}]", 15, &v) != 0);
    CHECK(json_load("[{\"title\":\"x\",\"priority\":\"Nope\",\"status\":\"Todo\"}]", 49, &v) != 0);
    CHECK(json_load("[1,2", 4, &v) != 0);
    CHECK(json_load("[] x", 4, &v) != 0);
    sb_free(&out);
}

static void press(uint32_t c)
{
    app_key((Key){c, false});
}

static void ctrl(uint32_t c)
{
    app_key((Key){c, true});
}

static void type(const char *s)
{
    for (; *s; s++)
        press((unsigned char)*s);
}

static Item make_item(const char *title, int prio, int status, const char *tag)
{
    Item it = {xstrdup(title), xstrdup(""), NULL, (uint8_t)prio, (uint8_t)status};

    if (tag)
        arr_push(it.tags, xstrdup(tag));
    return it;
}

static char test_dir[256];

static void open_app(void)
{
    Config cfg;
    Item *items = NULL;

    snprintf(test_dir, sizeof test_dir, "/tmp/hopes-test-%d", (int)getpid());
    memset(&cfg, 0, sizeof cfg);
    cfg.dir = xstrdup(test_dir);
    CHECK(store_open(&cfg) == 0);

    arr_push(items, make_item("Write compiler backend", P_URGENT, ST_TODO, "rust"));
    arr_push(items, make_item("Design responsive TUI", P_HIGH, ST_TODO, "ui"));
    arr_push(items, make_item("Implement undo buffer", P_MED, ST_TODO, "rust"));
    CHECK(store_save(items) == 0);
    items_free(items);
    app_init(&cfg);
    free(cfg.dir);
}

static void close_app(void)
{
    char cmd[300];

    app_free();
    snprintf(cmd, sizeof cmd, "rm -rf %s", test_dir);
    CHECK(system(cmd) == 0);
}

static Item *saved(void)
{
    Item *v = NULL;

    CHECK(store_load(&v) == 0);
    return v;
}

static void test_navigation(void)
{
    open_app();
    CHECK(app_selected() == 0);
    press('j');
    CHECK(app_selected() == 1);
    press('G');
    CHECK(app_selected() == 2);
    press('j');
    CHECK(app_selected() == 0);
    press('k');
    CHECK(app_selected() == 2);
    type("gg");
    CHECK(app_selected() == 0);
    press(HK_TAB);
    CHECK(A.focus == F_INSPECTOR);
    press(HK_TAB);
    CHECK(A.focus == F_NAV);
    press(HK_BACKTAB);
    CHECK(A.focus == F_INSPECTOR);
    close_app();
}

static void test_chords(void)
{
    open_app();
    press('d');
    CHECK(app_chord_label() != NULL);
    press('j');
    CHECK(app_chord_label() == NULL);
    press('d');
    CHECK(len_of(A.items) == 3);
    close_app();
}

static void test_status_priority(void)
{
    open_app();
    press(' ');
    CHECK(A.items[0].status == ST_WIP);
    press(' ');
    press(' ');
    CHECK(A.items[0].status == ST_TODO);
    press('p');
    CHECK(A.items[0].prio == P_HIGH);
    press('3');
    CHECK(A.items[0].prio == P_MED);
    press('P');
    CHECK(A.items[0].prio == P_HIGH);
    Item *v = saved();
    CHECK(v[0].prio == P_HIGH);
    items_free(v);
    close_app();
}

static void test_delete_undo_reorder(void)
{
    open_app();
    type("dd");
    CHECK(len_of(A.items) == 2);
    CHECK_STR(A.items[0].title, "Design responsive TUI");
    press('u');
    CHECK(len_of(A.items) == 3);
    CHECK_STR(A.items[0].title, "Write compiler backend");
    press('J');
    CHECK_STR(A.items[1].title, "Write compiler backend");
    CHECK(app_selected() == 1);
    press('K');
    CHECK_STR(A.items[0].title, "Write compiler backend");
    press('u');
    CHECK_STR(A.status, "Nothing to undo");
    close_app();
}

static void test_create_and_edit(void)
{
    open_app();
    press('a');
    CHECK(A.mode == M_CREATE);
    type("Ship it");
    press(HK_TAB);
    type("two words");
    press(HK_TAB);
    press('1');
    press(HK_TAB);
    type("a, b ,, c");
    ctrl('s');
    CHECK(A.mode == M_NORMAL);
    CHECK(len_of(A.items) == 4);
    Item *it = &A.items[3];
    CHECK_STR(it->title, "Ship it");
    CHECK_STR(it->desc, "two words");
    CHECK(it->prio == P_URGENT && len_of(it->tags) == 3);
    CHECK(app_selected() == 3);

    press('e');
    CHECK(A.mode == M_EDIT);
    ctrl('u');
    type("Renamed");
    press(HK_ENTER);
    CHECK_STR(A.items[3].title, "Renamed");

    press('a');
    press(HK_ENTER);
    CHECK(A.mode == M_CREATE);
    press(HK_ESC);
    press(HK_ESC);
    CHECK(A.mode == M_NORMAL);
    CHECK(len_of(A.items) == 4);

    Item *v = saved();
    CHECK(len_of(v) == 4);
    CHECK_STR(v[3].title, "Renamed");
    items_free(v);
    close_app();
}

static void test_filter_sort_tags(void)
{
    open_app();
    press('/');
    type("rust");
    press(HK_ENTER);
    CHECK(len_of(A.vis) == 2);
    press(HK_ESC);
    CHECK(len_of(A.vis) == 3);
    press('t');
    CHECK(A.filter != NULL && !strcmp(A.filter, "#rust"));
    CHECK(len_of(A.vis) == 2);
    press('t');
    press('t');
    CHECK(A.filter == NULL);
    press('s');
    CHECK(A.sort == SORT_PRIORITY);
    CHECK(A.vis[0] == 0);
    press('s');
    press('s');
    CHECK(A.sort == SORT_TITLE);
    CHECK_STR(A.items[A.vis[0]].title, "Design responsive TUI");
    close_app();
}

static void test_nav_and_kanban(void)
{
    open_app();
    press(HK_TAB);
    press(HK_TAB);
    CHECK(A.focus == F_NAV);
    press('5');
    CHECK(A.section == NAV_URGENT && len_of(A.vis) == 1);
    press(HK_ESC);
    CHECK(A.section == NAV_ALL);

    press(HK_TAB);
    press(HK_TAB);
    press('v');
    CHECK(A.view == V_KANBAN);
    press('L');
    CHECK(A.items[0].status == ST_WIP && A.kb_col == 1);
    press('H');
    CHECK(A.items[0].status == ST_TODO && A.kb_col == 0);
    press('l');
    CHECK(A.kb_col == 1);
    press('a');
    type("In flight");
    ctrl('s');
    CHECK(A.items[3].status == ST_WIP);
    close_app();
}

static void test_commands(void)
{
    open_app();
    press(':');
    type("nope");
    press(HK_ENTER);
    CHECK(strstr(A.status, "Unknown command") != NULL);
    press(':');
    type("q");
    press(HK_ENTER);
    CHECK(A.quit);
    close_app();
}

static void test_corrupt_file_is_preserved(void)
{
    char file[300];
    FILE *f;

    open_app();
    snprintf(file, sizeof file, "%s/items.json", test_dir);
    f = fopen(file, "w");
    fputs("{not json", f);
    fclose(f);
    Item *v = NULL;
    CHECK(store_load(&v) != 0);
    CHECK(access(file, F_OK) != 0);
    close_app();
}

static void make_dir(const char *path)
{
    char cmd[400];

    snprintf(cmd, sizeof cmd, "rm -rf %s && mkdir -p %s", path, path);
    CHECK(system(cmd) == 0);
}

static void test_exec_backend(void)
{
    char root[128], script[200];
    Config cfg;
    Item *items = NULL, *loaded = NULL;
    FILE *f;

    snprintf(root, sizeof root, "/tmp/hopes-exec-%d", (int)getpid());
    make_dir(root);
    snprintf(script, sizeof script, "%s/store.sh", root);
    f = fopen(script, "w");
    fputs("#!/bin/sh\nf=\"$(dirname \"$0\")/data.json\"\ncase \"$1\" in\n"
          "load) cat \"$f\" 2>/dev/null || true ;;\nsave) cat > \"$f\" ;;\nesac\n",
          f);
    fclose(f);
    chmod(script, 0755);

    memset(&cfg, 0, sizeof cfg);
    cfg.backend = BE_EXEC;
    cfg.dir = xstrdup(root);
    arr_push(cfg.command, xstrdup(script));
    CHECK(store_open(&cfg) == 0);
    CHECK(store_load(&loaded) == 0 && len_of(loaded) == 0);
    arr_push(items, make_item("through a script", P_HIGH, ST_WIP, "exec"));
    CHECK(store_save(items) == 0);
    CHECK(store_load(&loaded) == 0 && len_of(loaded) == 1);
    CHECK_STR(loaded[0].title, "through a script");
    CHECK(loaded[0].prio == P_HIGH && loaded[0].status == ST_WIP);

    tags_free(cfg.command);
    cfg.command = NULL;
    arr_push(cfg.command, xstrdup("/usr/bin/false"));
    CHECK(store_open(&cfg) == 0);
    CHECK(store_save(items) != 0);
    CHECK(strstr(store_error(), "failed") != NULL);

    items_free(items);
    items_free(loaded);
    tags_free(cfg.command);
    free(cfg.dir);
    make_dir(root);
}

#ifdef HOPES_SQLITE
static void test_sqlite_backend(void)
{
    char root[128];
    Config cfg;
    Item *items = NULL, *loaded = NULL;

    snprintf(root, sizeof root, "/tmp/hopes-sqlite-%d", (int)getpid());
    make_dir(root);
    memset(&cfg, 0, sizeof cfg);
    cfg.backend = BE_SQLITE;
    cfg.dir = xstrdup(root);
    CHECK(store_open(&cfg) == 0);
    arr_push(items, make_item("first", P_LOW, ST_TODO, "a"));
    arr_push(items, make_item("second \"quoted\" 雨音", P_URGENT, ST_DONE, NULL));
    CHECK(store_save(items) == 0);
    CHECK(store_load(&loaded) == 0 && len_of(loaded) == 2);
    CHECK_STR(loaded[0].title, "first");
    CHECK_STR(loaded[1].title, "second \"quoted\" 雨音");
    CHECK(loaded[1].prio == P_URGENT && loaded[1].status == ST_DONE);
    items_free(items);
    items_free(loaded);
    free(cfg.dir);
    make_dir(root);
}
#endif

int main(void)
{
    test_motions();
    test_edits();
    test_empty_buffer();
    test_yank_paste();
    test_utf8();
    test_text_input_modes();
    test_json_roundtrip();
    test_json_edge_cases();
    test_navigation();
    test_chords();
    test_status_priority();
    test_delete_undo_reorder();
    test_create_and_edit();
    test_filter_sort_tags();
    test_nav_and_kanban();
    test_commands();
    test_corrupt_file_is_preserved();
    test_exec_backend();
#ifdef HOPES_SQLITE
    test_sqlite_backend();
#endif
    printf("%d checks, %d failed\n", checks, failures);
    return failures != 0;
}
