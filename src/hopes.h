#ifndef HOPES_H
#define HOPES_H

#if !defined(__APPLE__) && !defined(_GNU_SOURCE)
#define _GNU_SOURCE
#endif
#ifndef _XOPEN_SOURCE_EXTENDED
#define _XOPEN_SOURCE_EXTENDED 1
#endif

#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/stat.h>
#include <time.h>
#include <wchar.h>

#ifdef __APPLE__
#include <malloc/malloc.h>
#elif defined(__GLIBC__)
#include <malloc.h>
#endif

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <direct.h>
#include <io.h>
#include <windows.h>
#else
#include <poll.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <unistd.h>
#endif

#ifndef O_BINARY
#define O_BINARY 0
#endif

#define DYNAMIC_ARRAYS
#define STRING_BUILDER
#define UTF8
#include "amane.h"

#define len_of(a) ((int)arr_len(a))

void say(int fd, const char *msg);
void *xmalloc(size_t n);
void *xrealloc(void *p, size_t n);
char *xstrdup(const char *s);
char *xstrndup(const char *s, size_t n);
int64_t now_ms(void);
void heap_trim(void);

#define PLAT_INPUT 1
#define PLAT_SYNC 2
#define PLAT_HUP 4

void plat_init(void);
char *plat_base_dir(int config);
char *plat_state_dir(void);
void plat_mkdir(const char *path);
int plat_replace(const char *from, const char *to);
int plat_fsync(int fd);
void plat_localtime(time_t t, struct tm *out);
int plat_wait(int timeout_ms, intptr_t sync_handle);
int plat_run(char *const argv[], const char *dir, const String_Builder *input, String_Builder *out,
             String_Builder *err, size_t limit, int timeout_ms);

void snippet(char *out, size_t cap, const char *title, int max);

enum { P_URGENT, P_HIGH, P_MED, P_LOW };
enum { ST_TODO, ST_WIP, ST_DONE };

typedef int32_t Day;

enum { NO_DAY = 0 };
enum { R_NONE, R_EVERY, R_ON };
enum { RU_DAY, RU_WEEK, RU_MONTH, RU_YEAR };
typedef enum { U_OVERDUE, U_TODAY, U_SOON, U_LATER, U_UPCOMING } Urgency;

typedef struct {
    uint8_t kind, unit, mask;
    uint16_t n;
} Repeat;

typedef struct {
    Day start, end;
} Window;

typedef struct {
    char *title, *desc;
    char **tags;
    uint8_t prio, status;
    Day due, start;
    Repeat repeat;
} Item;

Day day_make(int64_t y, int64_t m, int64_t d);
void day_civil(Day day, int *y, int *m, int *d);
int day_weekday(Day day);
Day day_today(void);
Day day_add(Day day, int64_t n);
Day day_parse_iso(const char *s);
void day_format_iso(Day day, char out[11]);
void day_format_us(Day day, char out[11]);
int day_parse(const char *input, Day today, Day *out);
int window_parse(const char *input, Day today, Window *w);
Urgency day_urgency(Day due, Day today);
Urgency window_urgency(const Window *w, Day today);
void day_relative_label(Day due, Day today, char *out, size_t cap);
void window_label(const Window *w, Day today, char *out, size_t cap);
void window_full_label(const Window *w, char *out, size_t cap);
Window window_moved_to(const Window *w, Day end);
int item_window(const Item *it, Window *w);
int item_overdue(const Item *it, Day today);
int repeat_parse(const char *input, Repeat *r);
void repeat_format(const Repeat *r, char *out, size_t cap);
Day repeat_first_from(const Repeat *r, Day from);
Day repeat_next_after(const Repeat *r, Day base, Day today);

void item_free(Item *it);
void items_free(Item *v);
void tags_free(char **tags);

const char *prio_label(int p);
const char *prio_short(int p);
const char *prio_glyph(int p);
int prio_next(int p);
int prio_prev(int p);
const char *status_label(int s);
const char *status_badge(int s);

int json_load(const char *data, size_t len, Item **out);
void json_dump(String_Builder *out, const Item *v, int n, int pretty);

enum { BE_JSON, BE_SQLITE, BE_EXEC };

typedef struct {
    int sync_enabled;
    int debounce;
    char *sync_dir;
    char *remote, *branch;
    int pull_on_startup;
    int backend;
    char *dir;
    char **command;
} Config;

void config_load(Config *c);
char *default_data_dir(void);

int store_open(const Config *c);
int store_load(Item **v);
int store_save(const Item *v);
const char *store_dir(void);
const char *store_error(void);

typedef enum {
    SY_DISABLED,
    SY_IDLE,
    SY_SYNCING,
    SY_SYNCED,
    SY_UPDATED,
    SY_OFFLINE,
    SY_CONFLICT,
    SY_ERROR
} Sync_Kind;

typedef void (*Sync_Cb)(Sync_Kind kind, const char *msg);

extern int sync_enabled;
void sync_init(const char *dir, const Config *c, int enabled, Sync_Cb cb);
intptr_t sync_handle(void);
int sync_flush_main(const char *dir, const char *remote, const char *branch);
void sync_pump(void);
void sync_request_push(void);
void sync_force(void);
int sync_timeout_ms(void);
void sync_timers(void);
void sync_shutdown(void);

typedef struct {
    uint32_t c;
    int ctrl;
} Key;

enum {
    HK_ESC = 0x200001,
    HK_ENTER,
    HK_TAB,
    HK_BACKTAB,
    HK_BACKSPACE,
    HK_UP,
    HK_DOWN,
    HK_LEFT,
    HK_RIGHT,
    HK_HOME,
    HK_END,
    HK_PGUP,
    HK_PGDN,
    HK_F5
};

size_t te_clamp_normal(const String_Builder *b, size_t cur);
size_t te_move_left(const String_Builder *b, size_t cur);
size_t te_move_right(const String_Builder *b, size_t cur);
size_t te_move_up(const String_Builder *b, size_t cur);
size_t te_move_down(const String_Builder *b, size_t cur);
size_t te_caret_left(const String_Builder *b, size_t cur);
size_t te_caret_right(const String_Builder *b, size_t cur);
size_t te_caret_up(const String_Builder *b, size_t cur);
size_t te_caret_down(const String_Builder *b, size_t cur);
size_t te_line_start(const String_Builder *b, size_t cur);
size_t te_line_end(const String_Builder *b, size_t cur);
size_t te_word_forward(const String_Builder *b, size_t cur);
size_t te_word_end(const String_Builder *b, size_t cur);
size_t te_word_back(const String_Builder *b, size_t cur);
size_t te_insert(String_Builder *b, size_t cur, const char *s, size_t n);
size_t te_backspace(String_Builder *b, size_t cur);
size_t te_delete_char(String_Builder *b, size_t cur);
size_t te_delete_to_line_end(String_Builder *b, size_t cur);
size_t te_delete_line(String_Builder *b, size_t cur);
size_t te_insert_after(const String_Builder *b, size_t cur);
size_t te_insert_line_end(const String_Builder *b, size_t cur);
size_t te_open_below(String_Builder *b, size_t cur, int allow_nl);
size_t te_open_above(String_Builder *b, size_t cur, int allow_nl);
char *te_yank(const String_Builder *b, size_t lo, size_t hi);
size_t te_delete_range(String_Builder *b, size_t lo, size_t hi);
size_t te_paste_after(String_Builder *b, size_t cur, const char *s);
size_t te_paste_before(String_Builder *b, size_t cur, const char *s);
size_t te_delete_word_back(String_Builder *b, size_t cur);

typedef struct {
    size_t cursor;
    int insert, pending_d;
    long anchor;
    char *clip;
} Text_Input;

void ti_init(Text_Input *t);
void ti_focus(Text_Input *t, const String_Builder *b);
int ti_esc(Text_Input *t, String_Builder *b);
void ti_key(Text_Input *t, String_Builder *b, Key k, int allow_nl);

typedef enum { M_NORMAL, M_CREATE, M_EDIT, M_FIND, M_CMD, M_HELP } Mode;
typedef enum { V_TABLE, V_KANBAN } View_Mode;
typedef enum { F_NAV, F_MAIN, F_INSPECTOR } Focus;
typedef enum { SORT_NATURAL, SORT_PRIORITY, SORT_STATUS, SORT_DUE, SORT_TITLE, SORT_COUNT } Sort;
typedef enum { NAV_ALL, NAV_TODO, NAV_WIP, NAV_DONE, NAV_URGENT, NAV_OVERDUE, NAV_COUNT } Nav_Section;
typedef enum { FF_TITLE, FF_DESC, FF_PRIO, FF_DUE, FF_REPEAT, FF_TAGS, FF_COUNT } Form_Field;

typedef struct {
    int total, todo, wip, done, percent, overdue;
    int prio[4];
} Stats;

typedef struct {
    const char *name;
    int count;
} Tag_Count;

typedef struct {
    time_t at;
    char msg[160];
} Audit_Entry;

#define AUDIT_MAX 50
#define NAV_TAGS_SHOWN 6

typedef struct {
    String_Builder title, desc, tags, due, repeat;
    int prio, tag_sel;
    Form_Field field;
    int edit_idx;
    int create_status;
    Text_Input in;
} Editor;

typedef struct {
    int find;
    String_Builder buf;
    Text_Input in;
} Cmd_Line;

typedef struct {
    Item *items;
    Day today;
    unsigned rev;

    View_Mode view;
    Focus focus;
    Sort sort;
    Nav_Section section;
    char *filter;
    Item deleted;
    int deleted_idx;
    int has_deleted;

    Sync_Kind sync;
    char sync_msg[160];
    Audit_Entry audit[AUDIT_MAX];
    int naudit;
    char status[200];
    int64_t status_until;

    int *vis;
    int *col[3];
    Tag_Count *tags;
    Stats stats;

    Mode mode;
    int quit;

    int tbl_sel, tbl_off;
    int chord_g, chord_d;
    int kb_col, kb_cur[3], kb_off[3];
    int nav_idx;
    int insp_scroll, insp_total, insp_vis;
    int help_scroll, help_max;
    Editor ed;
    Cmd_Line cmd;
} App;

extern App A;

void app_init(const Config *cfg);
void app_free(void);
void app_key(Key k);
void app_sync_status(Sync_Kind kind, const char *msg);
void app_refresh(void);
const char *app_chord_label(void);
int app_selected(void);
int app_position(void);
void app_set_status(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
void app_log(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
const char *nav_label(int section);
void app_shutdown(void);
const char **ed_suggestions(void);
int ed_tag_target(void);
void view_encode(String_Builder *out);
int view_decode(const char *data, size_t len);
void view_save(void);
void view_load(void);

void ui_init(void);
void ui_end(void);
void ui_render(void);

int ui_read_input(void);

#ifdef HOPES_IMPLEMENTATION
#include "util.h"
#include "platform.h"
#include "dates.h"
#include "textedit.h"
#include "json.h"
#include "config.h"
#include "store.h"
#include "sync.h"
#include "app.h"
#include "view.h"
#ifndef HOPES_NO_UI
#include "ui.h"
#endif
#endif

#endif
