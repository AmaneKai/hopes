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
    CHECK(te_open_below(&s, 0, 0) == 5);
    CHECK_STR(s.data, "title");
    sb_free(&s);

    s = mk("one");
    CHECK(te_open_below(&s, 1, 1) == 4);
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
    ti_key(&t, &b, (Key){'0', 0}, 0);
    ti_key(&t, &b, (Key){'v', 0}, 0);
    ti_key(&t, &b, (Key){'e', 0}, 0);
    ti_key(&t, &b, (Key){'y', 0}, 0);
    CHECK_STR(t.clip, "hello");
    ti_key(&t, &b, (Key){'$', 0}, 0);
    ti_key(&t, &b, (Key){'p', 0}, 0);
    CHECK_STR(b.data, "hello worldhello");
    ti_key(&t, &b, (Key){'d', 0}, 0);
    ti_key(&t, &b, (Key){'d', 0}, 0);
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

    json_dump(&out, v, 1, 1);
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
    json_dump(&out, v, 0, 1);
    CHECK_STR(out.data, "[]");
    CHECK(json_load("[{\"title\":\"x\"}]", 15, &v) != 0);
    CHECK(json_load("[{\"title\":\"x\",\"priority\":\"Nope\",\"status\":\"Todo\"}]", 49, &v) != 0);
    CHECK(json_load("[1,2", 4, &v) != 0);
    CHECK(json_load("[] x", 4, &v) != 0);
    sb_free(&out);
}

static void press(uint32_t c)
{
    app_key((Key){c, 0});
}

static void ctrl(uint32_t c)
{
    app_key((Key){c, 1});
}

static void type(const char *s)
{
    for (; *s; s++)
        press((unsigned char)*s);
}

static Item make_item(const char *title, int prio, int status, const char *tag)
{
    Item it = {.title = xstrdup(title), .desc = xstrdup(""), .prio = (uint8_t)prio, .status = (uint8_t)status};

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
    press(HK_TAB);
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
    CHECK(A.sort == SORT_DUE);
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

#define TODAY day_make(2026, 10, 25)

static Day pd(const char *text)
{
    Day out = NO_DAY;

    return day_parse(text, TODAY, &out) ? NO_DAY : out;
}

static void check_label(Day due, const char *want)
{
    char buf[48];

    day_relative_label(due, TODAY, buf, sizeof buf);
    CHECK_STR(buf, want);
}

static void test_calendar(void)
{
    int y, m, d;

    CHECK(day_make(1, 1, 1) == 1);
    CHECK(day_make(9999, 12, 31) == MAX_DAY);
    CHECK(day_make(2026, 2, 29) == NO_DAY);
    CHECK(day_make(2028, 2, 29) != NO_DAY);
    CHECK(day_make(1900, 2, 29) == NO_DAY);
    CHECK(day_make(2000, 2, 29) != NO_DAY);
    CHECK(day_make(0, 1, 1) == NO_DAY);
    CHECK(day_make(2026, 13, 1) == NO_DAY);
    CHECK(day_make(2026, 1, 0) == NO_DAY);
    CHECK(day_weekday(day_make(2026, 10, 25)) == 6);
    CHECK(day_weekday(day_make(2026, 10, 26)) == 0);
    for (Day day = 1; day <= MAX_DAY; day += 997) {
        day_civil(day, &y, &m, &d);
        CHECK(day_make(y, m, d) == day);
    }
    CHECK(day_add(day_make(2026, 1, 31), 1) == day_make(2026, 2, 1));
    CHECK(day_add(1, -1) == NO_DAY);
    CHECK(day_add(MAX_DAY, 1) == NO_DAY);
}

static void test_date_input(void)
{
    CHECK(pd("10-30-26") == day_make(2026, 10, 30) && pd("10/30/26") == day_make(2026, 10, 30));
    CHECK(pd("10-30-2026") == day_make(2026, 10, 30) && pd(" 1-5-27 ") == day_make(2027, 1, 5));
    CHECK(pd("10-25") == TODAY && pd("10-26") == day_make(2026, 10, 26));
    CHECK(pd("10-24") == day_make(2027, 10, 24) && pd("12/25") == day_make(2026, 12, 25));
    CHECK(pd("02-29-28") == day_make(2028, 2, 29) && pd("02-29-27") == NO_DAY);
    CHECK(pd("") == NO_DAY && pd("none") == NO_DAY && pd("-") == NO_DAY);
    CHECK(pd("13-01-26") == NO_DAY && pd("00-10-26") == NO_DAY && pd("10-32-26") == NO_DAY);
    CHECK(pd("10-30-2") == NO_DAY && pd("10-30-202") == NO_DAY && pd("10-30-0000") == NO_DAY);
    CHECK(pd("10") == NO_DAY && pd("10-30-26-1") == NO_DAY && pd("10--26") == NO_DAY);
    CHECK(pd("fri") == NO_DAY && pd("tomorrow") == NO_DAY && pd("+3d") == NO_DAY && pd("oct 25") == NO_DAY);
    CHECK(pd("2026-10-30") == NO_DAY && pd("99999999999-1-1") == NO_DAY);
}

static void test_date_windows(void)
{
    Window w;

    CHECK(window_parse("10-30-26..11-01-26", TODAY, &w) == 1);
    CHECK(w.start == day_make(2026, 10, 30) && w.end == day_make(2026, 11, 1));
    CHECK(window_parse("10-26-26 to 10-30-26", TODAY, &w) == 1);
    CHECK(w.start == day_make(2026, 10, 26) && w.end == day_make(2026, 10, 30));
    CHECK(window_parse("10-30 - 11-02", TODAY, &w) == 1);
    CHECK(w.start == day_make(2026, 10, 30) && w.end == day_make(2026, 11, 2));
    CHECK(window_parse("10-30-26 " EN_DASH " 11-02-26", TODAY, &w) == 1 && w.start == day_make(2026, 10, 30));
    CHECK(window_parse("10-30-26..10-30-26", TODAY, &w) == 1 && w.start == NO_DAY && w.end == day_make(2026, 10, 30));
    CHECK(window_parse("11-05-26..11-02-26", TODAY, &w) < 0);
    CHECK(window_parse("11-05..11-02", TODAY, &w) == 1 && w.end == day_make(2027, 11, 2));
    CHECK(window_parse("..10-30-26", TODAY, &w) < 0 && window_parse("10-30-26..", TODAY, &w) < 0);
    CHECK(window_parse("10-31-26", TODAY, &w) == 1 && w.start == NO_DAY && w.end == day_make(2026, 10, 31));
    CHECK(window_parse("  ", TODAY, &w) == 0 && window_parse("clear", TODAY, &w) == 0);
    CHECK(window_parse("fri..sun", TODAY, &w) < 0 && window_parse("nonsense", TODAY, &w) < 0);
}

static void test_date_labels(void)
{
    char buf[64];
    Window w = {NO_DAY, day_make(2026, 10, 30)};

    check_label(day_make(2026, 10, 20), "5d late");
    check_label(day_make(2026, 10, 24), "yesterday");
    check_label(TODAY, "today");
    check_label(day_make(2026, 10, 26), "tomorrow");
    check_label(day_make(2026, 10, 30), "Fri");
    check_label(day_make(2026, 11, 8), "11-08-26");
    check_label(day_make(2027, 1, 5), "01-05-27");
    day_format_us(day_make(1999, 12, 31), buf);
    CHECK_STR(buf, "12-31-1999");
    CHECK(pd(buf) == day_make(1999, 12, 31));
    day_format_us(day_make(2100, 1, 1), buf);
    CHECK_STR(buf, "01-01-2100");
    window_label(&w, TODAY, buf, sizeof buf);
    CHECK_STR(buf, "Fri");
    w.start = day_make(2026, 10, 24);
    window_label(&w, TODAY, buf, sizeof buf);
    CHECK_STR(buf, "5d left");
    w.start = day_make(2026, 10, 27);
    window_full_label(&w, buf, sizeof buf);
    CHECK_STR(buf, "10-27-26 " EN_DASH " 10-30-26");
    w.start = day_make(2026, 10, 29);
    window_label(&w, TODAY, buf, sizeof buf);
    CHECK_STR(buf, "from Thu");
    w.start = NO_DAY;
    window_full_label(&w, buf, sizeof buf);
    CHECK_STR(buf, "10-30-26");
    CHECK(day_urgency(day_make(2026, 10, 24), TODAY) == U_OVERDUE);
    CHECK(day_urgency(TODAY, TODAY) == U_TODAY);
    CHECK(day_urgency(day_make(2026, 10, 27), TODAY) == U_SOON);
    CHECK(day_urgency(day_make(2026, 10, 28), TODAY) == U_LATER);
    w.start = day_make(2026, 10, 29);
    CHECK(window_urgency(&w, TODAY) == U_UPCOMING);
    w = window_moved_to(&w, day_make(2026, 11, 6));
    CHECK(w.end == day_make(2026, 11, 6) && w.start == day_make(2026, 11, 5));
}

static void check_repeat(const char *text, int kind, const char *want)
{
    Repeat r;
    char buf[64];

    CHECK(repeat_parse(text, &r) == 0 && r.kind == kind);
    repeat_format(&r, buf, sizeof buf);
    CHECK_STR(buf, want);
}

static void test_repeat_rules(void)
{
    Repeat r;

    check_repeat("daily", R_EVERY, "daily");
    check_repeat("Every Week", R_EVERY, "weekly");
    check_repeat("annually", R_EVERY, "yearly");
    check_repeat("2w", R_EVERY, "every 2w");
    check_repeat("every 3 months", R_EVERY, "every 3m");
    check_repeat("1d", R_EVERY, "daily");
    check_repeat("weekdays", R_ON, "weekdays");
    check_repeat("weekend", R_ON, "weekends");
    check_repeat("every fri", R_ON, "every Fri");
    check_repeat("wed, mon", R_ON, "Mon, Wed");
    check_repeat("", R_NONE, "");
    check_repeat("none", R_NONE, "");
    CHECK(repeat_parse("every", &r) != 0 && repeat_parse("0d", &r) != 0 && repeat_parse("1000d", &r) != 0);
    CHECK(repeat_parse("funday", &r) != 0 && repeat_parse("5", &r) != 0 && repeat_parse("fri, nope", &r) != 0);
}

static void test_repeat_scheduling(void)
{
    Repeat r;

    repeat_parse("weekly", &r);
    CHECK(repeat_next_after(&r, day_make(2026, 10, 20), TODAY) == day_make(2026, 10, 27));
    CHECK(repeat_next_after(&r, day_make(2026, 11, 3), TODAY) == day_make(2026, 11, 10));
    repeat_parse("2d", &r);
    CHECK(repeat_next_after(&r, day_make(2026, 10, 1), TODAY) == day_make(2026, 10, 25));
    repeat_parse("monthly", &r);
    CHECK(repeat_next_after(&r, day_make(2026, 1, 31), TODAY) == day_make(2026, 10, 31));
    repeat_parse("yearly", &r);
    CHECK(repeat_next_after(&r, day_make(2024, 2, 29), TODAY) == day_make(2027, 2, 28));
    repeat_parse("every fri", &r);
    CHECK(repeat_next_after(&r, day_make(2026, 10, 30), TODAY) == day_make(2026, 11, 6));
    CHECK(repeat_first_from(&r, TODAY) == day_make(2026, 10, 30));
    repeat_parse("weekdays", &r);
    CHECK(repeat_first_from(&r, TODAY) == day_make(2026, 10, 26));
    repeat_parse("daily", &r);
    CHECK(repeat_first_from(&r, TODAY) == TODAY);
    CHECK(repeat_next_after(&r, MAX_DAY, TODAY) == NO_DAY);
}

static void test_json_schedule_fields(void)
{
    const char *src = "[{\"title\":\"t\",\"description\":\"\",\"priority\":\"Low\",\"status\":\"Todo\",\"tags\":[],"
                      "\"due\":\"2026-10-30\",\"start\":\"2026-10-28\",\"repeat\":\"every Fri\"}]";
    Item *v = NULL;
    String_Builder out = {0};

    CHECK(json_load(src, strlen(src), &v) == 0 && len_of(v) == 1);
    CHECK(v[0].due == day_make(2026, 10, 30) && v[0].start == day_make(2026, 10, 28));
    CHECK(v[0].repeat.kind == R_ON && v[0].repeat.mask == 16);
    json_dump(&out, v, 1, 0);
    CHECK_STR(out.data, src);
    items_free(v);
    sb_free(&out);

    src = "[{\"title\":\"t\",\"priority\":\"Low\",\"status\":\"Todo\",\"due\":null,\"repeat\":null}]";
    CHECK(json_load(src, strlen(src), &v) == 0 && v[0].due == NO_DAY && v[0].repeat.kind == R_NONE);
    json_dump(&out, v, 1, 0);
    CHECK(strstr(out.data, "due") == NULL && strstr(out.data, "repeat") == NULL);
    items_free(v);
    sb_free(&out);
}

static int loads(const char *json)
{
    Item *v = NULL;
    int rc = json_load(json, strlen(json), &v);

    items_free(v);
    return rc;
}

#define ITEM_HEAD "{\"title\":\"t\",\"priority\":\"Low\",\"status\":\"Todo\""

static void test_json_hardening(void)
{
    Item *v = NULL;
    const char *nul = "[" ITEM_HEAD ",\"description\":\"\\u0000x\"}]";

    CHECK(loads("[" ITEM_HEAD ",\"due\":\"2026-02-30\"}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"due\":\"30-10-2026\"}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"due\":\"2026-1-5\"}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"due\":5}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"repeat\":\"nonsense\"}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"repeat\":\"none\"}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"title\":\"again\"}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"extra\":abc}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"extra\":01}]") != 0);
    CHECK(loads("[" ITEM_HEAD ",\"extra\":-1.5e+3}]") == 0);
    CHECK(loads("[" ITEM_HEAD ",\"extra\":[true,false,null,{\"a\":[]}]}]") == 0);
    CHECK(loads("[" ITEM_HEAD ",\"extra\":nul}]") != 0);
    CHECK(loads("[{\"title\":\"\xff\",\"priority\":\"Low\",\"status\":\"Todo\"}]") != 0);
    CHECK(loads("[{\"title\":\"\xc0\xaf\",\"priority\":\"Low\",\"status\":\"Todo\"}]") != 0);
    CHECK(loads("[{\"title\":\"\xed\xa0\x80\",\"priority\":\"Low\",\"status\":\"Todo\"}]") != 0);
    CHECK(loads("[{\"title\":\"\xe2\x80\x93\",\"priority\":\"Low\",\"status\":\"Todo\"}]") == 0);
    CHECK(loads("[" ITEM_HEAD ",\"description\":\"\\ud800\"}]") != 0);
    CHECK(json_load(nul, strlen(nul), &v) == 0);
    CHECK_STR(v[0].desc, "\xef\xbf\xbdx");
    items_free(v);
}

static uint32_t rng_state = 0x9E3779B9u;

static uint32_t rng(void)
{
    rng_state ^= rng_state << 13;
    rng_state ^= rng_state >> 17;
    rng_state ^= rng_state << 5;
    return rng_state;
}

static void test_fuzz_parsers(void)
{
    const char *seed = "[{\"title\":\"t \\u00e9\",\"description\":\"d\",\"priority\":\"Low\",\"status\":\"Todo\","
                       "\"tags\":[\"a\",\"b\"],\"due\":\"2026-10-30\",\"start\":\"2026-10-28\",\"repeat\":\"weekly\"}]";
    static const char alphabet[] = "0123456789 -/.,+abcdefghijklmnopqrstuvwxyz";
    size_t len = strlen(seed);

    for (int i = 0; i < 4000; i++) {
        char *copy = xstrdup(seed);
        Item *v = NULL;

        for (int k = 0, flips = 1 + (int)(rng() % 3); k < flips; k++)
            copy[rng() % len] = (char)(rng() & 0xFF ? rng() & 0xFF : 'x');
        if (json_load(copy, len, &v) == 0)
            items_free(v);
        free(copy);
    }
    for (int i = 0; i < 4000; i++) {
        char text[24];
        Window w;
        Repeat r;
        Day d;
        size_t n = rng() % (sizeof text - 1);

        for (size_t k = 0; k < n; k++)
            text[k] = alphabet[rng() % (sizeof alphabet - 1)];
        text[n] = 0;
        day_parse(text, TODAY, &d);
        window_parse(text, TODAY, &w);
        repeat_parse(text, &r);
    }
    CHECK(1);
}

static Day real_today(void)
{
    return day_today();
}

static void us_date(Day day, char *out)
{
    day_format_us(day, out);
}

static void fill_task(const char *title, const char *due, const char *repeat)
{
    press('a');
    type(title);
    press(HK_TAB);
    press(HK_TAB);
    press(HK_TAB);
    type(due);
    press(HK_TAB);
    type(repeat);
}

static void test_schedule_in_editor(void)
{
    Day today = real_today();
    char tomorrow[11], window[32], a[11], b[11];

    us_date(today + 1, tomorrow);
    us_date(today + 2, a);
    us_date(today + 4, b);
    snprintf(window, sizeof window, "%s..%s", a, b);
    open_app();
    fill_task("Water plants", tomorrow, "daily");
    CHECK(A.ed.field == FF_REPEAT);
    ctrl('s');
    CHECK(A.mode == M_NORMAL && len_of(A.items) == 4);
    CHECK(A.items[3].due == today + 1 && A.items[3].repeat.kind == R_EVERY);
    CHECK(A.items[3].start == NO_DAY);

    press('e');
    CHECK(A.mode == M_EDIT);
    CHECK(A.ed.due.count == 8 && A.ed.repeat.count == 5);
    CHECK_STR(sb_cstr(&A.ed.due), tomorrow);
    CHECK_STR(sb_cstr(&A.ed.repeat), "daily");
    press(HK_ESC);
    press(HK_ESC);
    CHECK(A.mode == M_NORMAL);

    fill_task("Bad date", "zzz", "");
    ctrl('s');
    CHECK(A.mode == M_CREATE && len_of(A.items) == 4);
    CHECK_STR(A.status, "Invalid due date");
    press(HK_ESC);
    press(HK_ESC);

    fill_task("Bad rule", "", "sometimes");
    ctrl('s');
    CHECK(A.mode == M_CREATE && len_of(A.items) == 4);
    CHECK_STR(A.status, "Invalid repeat rule");
    press(HK_ESC);
    press(HK_ESC);

    fill_task("Only repeat", "", "every mon");
    ctrl('s');
    CHECK(len_of(A.items) == 5);
    CHECK(A.items[4].due >= today && day_weekday(A.items[4].due) == 0 && A.items[4].due < today + 7);

    fill_task("Window", window, "");
    ctrl('s');
    CHECK(len_of(A.items) == 6 && A.items[5].start != NO_DAY && A.items[5].start < A.items[5].due);
    close_app();
}

static void test_repeat_rolls_forward(void)
{
    Day today = real_today();
    Item *disk;

    char tomorrow[11];

    us_date(today + 1, tomorrow);
    open_app();
    fill_task("Water plants", tomorrow, "daily");
    ctrl('s');
    A.tbl_sel = 3;
    press(' ');
    CHECK(A.items[3].status == ST_WIP && A.items[3].due == today + 1);
    press(' ');
    CHECK(A.items[3].status == ST_TODO && A.items[3].due == today + 2);
    CHECK(strstr(A.status, "Done") != NULL && strstr(A.status, "next") != NULL);
    CHECK(A.naudit > 0 && strstr(A.audit[0].msg, "Completed") != NULL);
    disk = saved();
    CHECK(disk[3].due == today + 2 && disk[3].status == ST_TODO && disk[3].repeat.kind == R_EVERY);
    items_free(disk);

    A.items[3].start = today + 1;
    A.items[3].due = today + 2;
    A.items[3].status = ST_WIP;
    press(' ');
    CHECK(A.items[3].start == today + 2 && A.items[3].due == today + 3);

    A.items[0].status = ST_WIP;
    A.tbl_sel = 0;
    press(' ');
    CHECK(A.items[0].status == ST_DONE && A.items[0].due == NO_DAY);
    close_app();
}

static void test_overdue_and_due_sort(void)
{
    Day today = real_today();

    open_app();
    A.items[0].due = today - 1;
    A.items[1].due = today + 5;
    A.items[2].due = today - 3;
    A.items[2].status = ST_DONE;
    app_refresh();
    CHECK(A.stats.overdue == 1);
    A.section = NAV_OVERDUE;
    app_refresh();
    CHECK(len_of(A.vis) == 1 && A.vis[0] == 0);
    A.section = NAV_ALL;
    A.sort = SORT_DUE;
    app_refresh();
    CHECK(A.vis[0] == 2 && A.vis[1] == 0 && A.vis[2] == 1);
    A.items[2].due = NO_DAY;
    app_refresh();
    CHECK(A.vis[0] == 0 && A.vis[1] == 1 && A.vis[2] == 2);
    close_app();
}

static void tag_field(void)
{
    press('a');
    type("Tagged");
    for (int i = 0; i < 5; i++)
        press(HK_TAB);
}

static void test_tag_autocomplete(void)
{
    const char **sug;

    open_app();
    tag_field();
    CHECK(A.ed.field == FF_TAGS);
    sug = ed_suggestions();
    CHECK(len_of(sug) == 2 && ed_tag_target() == -1);
    arr_free(sug);

    type("U");
    sug = ed_suggestions();
    CHECK(len_of(sug) == 2 && !strcmp(sug[0], "ui") && !strcmp(sug[1], "rust") && ed_tag_target() == 0);
    arr_free(sug);
    press(HK_TAB);
    CHECK(A.ed.field == FF_TAGS);
    CHECK_STR(sb_cstr(&A.ed.tags), "ui, ");

    type("ru");
    sug = ed_suggestions();
    CHECK(len_of(sug) == 1 && !strcmp(sug[0], "rust"));
    arr_free(sug);
    press(HK_ENTER);
    CHECK_STR(sb_cstr(&A.ed.tags), "ui, rust, ");
    sug = ed_suggestions();
    CHECK(len_of(sug) == 0);
    arr_free(sug);
    press(HK_TAB);
    CHECK(A.ed.field == FF_TITLE);

    press(HK_BACKTAB);
    CHECK(A.ed.field == FF_TAGS);
    for (int i = 0; i < 12; i++)
        press(HK_BACKSPACE);
    CHECK_STR(sb_cstr(&A.ed.tags), "");
    press(HK_DOWN);
    CHECK(A.ed.field == FF_TAGS && A.ed.tag_sel == 0);
    press(HK_UP);
    CHECK(A.ed.tag_sel == 1);
    press(HK_UP);
    CHECK(A.ed.tag_sel == 0);
    press(HK_ENTER);
    CHECK_STR(sb_cstr(&A.ed.tags), "rust, ");
    type("x");
    sug = ed_suggestions();
    CHECK(len_of(sug) == 0);
    arr_free(sug);
    press(HK_ESC);
    sug = ed_suggestions();
    CHECK(len_of(sug) == 0);
    arr_free(sug);
    press(HK_ESC);
    close_app();
}

static void test_tag_contains_ranking(void)
{
    const char **sug;

    open_app();
    arr_push(A.items[0].tags, xstrdup("brush"));
    app_refresh();
    tag_field();
    type("ru");
    sug = ed_suggestions();
    CHECK(len_of(sug) == 2 && !strcmp(sug[0], "rust") && !strcmp(sug[1], "brush"));
    arr_free(sug);
    press(HK_ESC);
    press(HK_ESC);
    close_app();
}

static void test_view_state_round_trip(void)
{
    String_Builder out = {0};
    const char *saved_json;

    open_app();
    A.view = V_KANBAN;
    A.sort = SORT_DUE;
    A.section = NAV_OVERDUE;
    A.filter = xstrdup("#rust");
    view_encode(&out);
    CHECK_STR(out.data, "{\"view_mode\":\"Kanban\",\"sort_mode\":\"Due\",\"nav_section\":\"Overdue\",\"tag_filter\":\"#rust\"}");

    A.view = V_TABLE;
    A.sort = SORT_NATURAL;
    A.section = NAV_ALL;
    free(A.filter);
    A.filter = NULL;
    CHECK(view_decode(out.data, out.count) == 0);
    CHECK(A.view == V_KANBAN && A.sort == SORT_DUE && A.section == NAV_OVERDUE);
    CHECK(A.filter && !strcmp(A.filter, "#rust") && A.nav_idx == NAV_COUNT);

    saved_json = "{\"view_mode\":\"Table\",\"sort_mode\":\"Title\",\"nav_section\":\"Todo\",\"tag_filter\":\"#gone\"}";
    CHECK(view_decode(saved_json, strlen(saved_json)) == 0);
    CHECK(A.view == V_TABLE && A.sort == SORT_TITLE && A.filter == NULL && A.nav_idx == NAV_TODO);

    saved_json = "{\"view_mode\":\"Table\",\"sort_mode\":\"Title\",\"nav_section\":\"All\",\"tag_filter\":\"free text\"}";
    CHECK(view_decode(saved_json, strlen(saved_json)) == 0 && !strcmp(A.filter, "free text"));

    saved_json = "{\"view_mode\":\"Kanban\",\"sort_mode\":\"Natural\",\"nav_section\":\"All\",\"tag_filter\":null}";
    CHECK(view_decode(saved_json, strlen(saved_json)) == 0 && A.filter == NULL && A.view == V_KANBAN);

    saved_json = "{\"view_mode\":\"Table\",\"sort_mode\":\"Natural\",\"nav_section\":\"All\"}";
    CHECK(view_decode(saved_json, strlen(saved_json)) == 0 && A.view == V_TABLE);

    CHECK(view_decode("{\"view_mode\":\"Table\"}", 21) != 0);
    CHECK(view_decode("{\"view_mode\":\"Cube\",\"sort_mode\":\"Natural\",\"nav_section\":\"All\"}", 63) != 0);
    CHECK(A.view == V_TABLE);
    CHECK(view_decode("not json", 8) != 0 && view_decode("", 0) != 0);
    sb_free(&out);
    close_app();
}

static void test_view_state_on_disk(void)
{
    char home[128], cmd[200];

    snprintf(home, sizeof home, "/tmp/hopes-home-%d", (int)getpid());
    snprintf(cmd, sizeof cmd, "rm -rf %s && mkdir -p %s", home, home);
    CHECK(system(cmd) == 0);
    setenv("HOME", home, 1);
    unsetenv("XDG_STATE_HOME");

    open_app();
    view_load();
    CHECK(A.view == V_TABLE);
    A.view = V_KANBAN;
    A.sort = SORT_PRIORITY;
    view_save();
    A.view = V_TABLE;
    A.sort = SORT_NATURAL;
    view_load();
    CHECK(A.view == V_KANBAN && A.sort == SORT_PRIORITY);
    close_app();
    snprintf(cmd, sizeof cmd, "rm -rf %s", home);
    CHECK(system(cmd) == 0);
}

#ifdef HOPES_SQLITE
static void test_sqlite_failed_save_keeps_data(void)
{
    char root[128], path[160];
    Config cfg;
    Item *items = NULL, *loaded = NULL;
    sqlite3 *other = NULL;

    snprintf(root, sizeof root, "/tmp/hopes-sqlite-fail-%d", (int)getpid());
    make_dir(root);
    memset(&cfg, 0, sizeof cfg);
    cfg.backend = BE_SQLITE;
    cfg.dir = xstrdup(root);
    CHECK(store_open(&cfg) == 0);
    arr_push(items, make_item("first", P_LOW, ST_TODO, NULL));
    arr_push(items, make_item("second", P_LOW, ST_TODO, NULL));
    CHECK(store_save(items) == 0);

    snprintf(path, sizeof path, "%s/hopes.db", root);
    CHECK(sqlite3_open(path, &other) == SQLITE_OK);
    CHECK(sqlite3_exec(other, "CREATE TRIGGER boom BEFORE INSERT ON items BEGIN SELECT RAISE(ABORT, 'boom'); END", 0, 0,
                       0) == SQLITE_OK);
    sqlite3_close(other);

    arr_push(items, make_item("third", P_LOW, ST_TODO, NULL));
    CHECK(store_save(items) != 0);
    CHECK(strstr(store_error(), "boom") != NULL);
    CHECK(store_load(&loaded) == 0 && len_of(loaded) == 2);
    CHECK_STR(loaded[0].title, "first");
    CHECK_STR(loaded[1].title, "second");

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
    test_calendar();
    test_date_input();
    test_date_windows();
    test_date_labels();
    test_repeat_rules();
    test_repeat_scheduling();
    test_json_schedule_fields();
    test_json_hardening();
    test_fuzz_parsers();
    test_schedule_in_editor();
    test_repeat_rolls_forward();
    test_overdue_and_due_sort();
    test_tag_autocomplete();
    test_tag_contains_ranking();
    test_view_state_round_trip();
    test_view_state_on_disk();
    test_corrupt_file_is_preserved();
    test_exec_backend();
#ifdef HOPES_SQLITE
    test_sqlite_backend();
    test_sqlite_failed_save_keeps_data();
#endif
    printf("%d checks, %d failed\n", checks, failures);
    return failures != 0;
}
