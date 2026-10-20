#define HOPES_IMPLEMENTATION
#include "hopes.h"

#define VERSION "0.1.0"

static volatile sig_atomic_t stop;
static bool dirty;

static void on_signal(int sig)
{
    (void)sig;
    stop = 1;
}

static void on_sync(Sync_Kind kind, const char *msg)
{
    app_sync_status(kind, msg);
    dirty = true;
}

static int next_deadline(void)
{
    int t = sync_timeout_ms();

    if (A.status[0]) {
        int64_t left = A.status_until - now_ms();
        int ms = left > 0 ? (int)left : 0;

        t = t < 0 || ms < t ? ms : t;
    }
    return t;
}

static void expire_status(void)
{
    if (!A.status[0] || now_ms() < A.status_until)
        return;
    A.status[0] = 0;
    dirty = true;
}

static bool wait_for_event(void)
{
    int events = plat_wait(next_deadline(), sync_handle());

    if (stop || (events & PLAT_HUP))
        return false;
    if (events & PLAT_SYNC)
        sync_pump();
    ui_read_input();
    dirty = true;
    expire_status();
    sync_timers();
    return !A.quit;
}

static int usage(const char *prog)
{
    say(2, "usage: ");
    say(2, prog);
    say(2, " [--version]\n");
    return 2;
}

int main(int argc, char **argv)
{
    Config cfg;

    plat_init();
    if (argc == 5 && !strcmp(argv[1], "--sync-flush"))
        return sync_flush_main(argv[2], argv[3], argv[4]);
    if (argc == 2 && !strcmp(argv[1], "--version")) {
        say(1, "hopes " VERSION "\n");
        return 0;
    }
    if (argc > 1)
        return usage(argv[0]);

    config_load(&cfg);
    if (store_open(&cfg)) {
        say(2, "hopes: ");
        say(2, store_error());
        say(2, "\n");
        return 1;
    }
    sync_init(store_dir(), &cfg, cfg.sync_enabled && cfg.backend == BE_JSON, on_sync);
    app_init(&cfg);

    signal(SIGTERM, on_signal);
#ifdef SIGHUP
    signal(SIGHUP, on_signal);
#endif
    ui_init();
    dirty = true;
    for (;;) {
        if (dirty) {
            ui_render();
            dirty = false;
        }
        if (!wait_for_event())
            break;
    }
    ui_end();
    app_shutdown();
    return 0;
}
