#include "hopes.h"

#define NET_TIMEOUT_MS 10000
#define OFFLINE_RETRY_MS 30000
#define EVENT_LINE_MAX 320
#define ABORT_WAIT_MS 3000

int sync_enabled;

static char *dir, *remote, *branch;
static int debounce_ms;
static int pull_startup;
static Sync_Cb cb;

static int worker_running;
static char line[EVENT_LINE_MAX];
static size_t line_len;
static int dirty;
static int64_t pending_at;
static int64_t retry_at;
static int force_next;
static int quiet;

typedef enum { G_OK, G_OFFLINE, G_CONFLICT, G_REJECTED, G_FAIL } Git_Rc;

typedef struct {
    int rc;
    char out[1024], err[4096];
} Git_Out;

static void copy_capped(char *dst, size_t cap, const String_Builder *sb)
{
    size_t n = sb->count < cap - 1 ? sb->count : cap - 1;

    memcpy(dst, sb_cstr(sb), n);
    dst[n] = 0;
}

static int git(Git_Out *o, ...) __attribute__((sentinel));

static int git(Git_Out *o, ...)
{
    char *argv[16] = {"git"};
    int argc = 1;
    va_list ap;
    String_Builder out = {0}, err = {0};
    Git_Out local;

    va_start(ap, o);
    for (char *a; argc < 15 && (a = va_arg(ap, char *));)
        argv[argc++] = a;
    va_end(ap);
    argv[argc] = NULL;

    if (!o)
        o = &local;
    o->rc = plat_run(argv, dir, NULL, &out, &err, sizeof o->err - 1, NET_TIMEOUT_MS);
    copy_capped(o->out, sizeof o->out, &out);
    copy_capped(o->err, sizeof o->err, &err);
    sb_free(&out);
    sb_free(&err);
    return o->rc;
}

static int has(const char *hay, const char *needle)
{
    for (; *hay; hay++)
        if (!strncasecmp(hay, needle, strlen(needle)))
            return 1;
    return 0;
}

static int is_offline(const char *e)
{
    return has(e, "could not resolve host") || has(e, "unable to access") ||
           has(e, "connection timed out") || has(e, "network is unreachable") ||
           has(e, "could not read from remote repository") || has(e, "no route to host");
}

static Git_Rc classify(Git_Out *o, char *msg, size_t cap)
{
    size_t n = strlen(o->err);

    if (o->rc == 0)
        return G_OK;
    if (o->rc == -2 || is_offline(o->err))
        return G_OFFLINE;
    if (strstr(o->err, "CONFLICT") || strstr(o->err, "could not apply"))
        return G_CONFLICT;
    if (strstr(o->err, "[rejected]") || strstr(o->err, "non-fast-forward") || strstr(o->err, "fetch first"))
        return G_REJECTED;
    while (n && (o->err[n - 1] == '\n' || o->err[n - 1] == '\r' || o->err[n - 1] == ' '))
        o->err[--n] = 0;
    snprintf(msg, cap, "%.255s", o->err[0] ? o->err : "git failed");
    return G_FAIL;
}

static int has_unsynced(void)
{
    Git_Out o;
    char range[256];

    git(&o, "status", "--porcelain", "--", "items.json", NULL);
    if (o.rc == 0 && o.out[0])
        return 1;
    snprintf(range, sizeof range, "%s/%s..HEAD", remote, branch);
    git(&o, "rev-list", "--count", range, NULL);
    return o.rc == 0 && atoi(o.out) > 0;
}

#ifdef _WIN32

static CRITICAL_SECTION queue_lock;
static HANDLE queue_event, worker_thread;
static char queue[4096];
static size_t queue_len;

#define NO_HANDLE 0

static void emit_raw(const char *text, size_t n)
{
    EnterCriticalSection(&queue_lock);
    if (queue_len + n > sizeof queue)
        queue_len = 0;
    if (n <= sizeof queue) {
        memcpy(queue + queue_len, text, n);
        queue_len += n;
    }
    LeaveCriticalSection(&queue_lock);
    SetEvent(queue_event);
}

static long worker_read(char *buf, size_t cap)
{
    size_t n;

    EnterCriticalSection(&queue_lock);
    n = queue_len < cap ? queue_len : cap;
    memcpy(buf, queue, n);
    memmove(queue, queue + n, queue_len - n);
    queue_len -= n;
    if (!queue_len)
        ResetEvent(queue_event);
    LeaveCriticalSection(&queue_lock);
    return (long)n;
}

intptr_t sync_handle(void)
{
    return worker_running ? (intptr_t)queue_event : NO_HANDLE;
}

#else

static int event_fd;
static pid_t worker_pid;

#define NO_HANDLE (-1)

static void emit_raw(const char *text, size_t n)
{
    if (event_fd > 0 && write(event_fd, text, n) < 0)
        return;
}

static long worker_read(char *buf, size_t cap)
{
    ssize_t n = read(event_fd, buf, cap);

    if (n < 0 && (errno == EINTR || errno == EAGAIN))
        return 0;
    return n > 0 ? (long)n : -1;
}

intptr_t sync_handle(void)
{
    return worker_running ? event_fd : NO_HANDLE;
}

#endif

static void emit(char code, const char *msg)
{
    char text[EVENT_LINE_MAX];
    int n;

    if (quiet)
        return;
    n = snprintf(text, sizeof text, "%c%s\n", code, msg ? msg : "");
    if (n >= (int)sizeof text) {
        n = sizeof text - 1;
        text[n - 1] = '\n';
    }
    emit_raw(text, (size_t)n);
}

static void copy_file(const char *from, const char *to)
{
    char chunk[4096];
    size_t n;
    FILE *in = fopen(from, "rb"), *out = fopen(to, "wb");

    while (in && out && (n = fread(chunk, 1, sizeof chunk, in)) > 0)
        fwrite(chunk, 1, n, out);
    if (in)
        fclose(in);
    if (out)
        fclose(out);
}

static void conflict_keep_remote(void)
{
    char src[1024], backup[1100], ref[256];
    Git_Out o;

    git(NULL, "rebase", "--abort", NULL);
    snprintf(src, sizeof src, "%s/items.json", dir);
    snprintf(backup, sizeof backup, "%s/items.json.local-backup-%ld", dir, (long)time(NULL));
    copy_file(src, backup);
    snprintf(ref, sizeof ref, "%s/%s", remote, branch);
    git(&o, "fetch", remote, branch, NULL);
    if (o.rc == 0)
        git(&o, "reset", "--hard", ref, NULL);
    if (o.rc == 0)
        emit('C', backup);
    else
        emit('E', o.err);
}

static void head_of(char *out, size_t cap)
{
    Git_Out o;

    git(&o, "rev-parse", "HEAD", NULL);
    snprintf(out, cap, "%.63s", o.out);
}

static int rebase_and_push(void)
{
    Git_Out o;
    char msg[256], before[64], after[64];
    Git_Rc r;

    emit('S', "Pulling...");
    head_of(before, sizeof before);
    git(&o, "pull", "--rebase", "--autostash", remote, branch, NULL);
    r = classify(&o, msg, sizeof msg);
    if (r == G_CONFLICT) {
        conflict_keep_remote();
        return 0;
    }
    if (r == G_OK) {
        head_of(after, sizeof after);
        if (strcmp(before, after))
            emit('U', NULL);
        git(&o, "push", remote, branch, NULL);
        r = classify(&o, msg, sizeof msg);
    }
    switch (r) {
    case G_OK: emit('Y', NULL); return 0;
    case G_OFFLINE: emit('O', NULL); return 1;
    case G_REJECTED: emit('E', "remote has newer commits"); return 0;
    default: emit('E', msg); return 0;
    }
}

static int commit_local(void)
{
    Git_Out o;

    emit('S', "Committing...");
    git(NULL, "add", "items.json", NULL);
    git(&o, "commit", "-m", "sync: update tasks", NULL);
    if (o.rc == 0 || strstr(o.out, "nothing to commit") || strstr(o.err, "nothing to commit"))
        return 1;
    emit('E', o.rc == -2 ? "git commit timed out" : o.err[0] ? o.err : "git commit failed");
    return 0;
}

static int do_push(int full)
{
    Git_Out o;
    char msg[256];

    if (!commit_local())
        return 0;
    if (full)
        return rebase_and_push();
    emit('S', "Pushing...");
    git(&o, "push", remote, branch, NULL);
    switch (classify(&o, msg, sizeof msg)) {
    case G_OK: emit('Y', NULL); return 0;
    case G_OFFLINE: emit('O', NULL); return 1;
    case G_REJECTED: return rebase_and_push();
    default: emit('E', msg); return 0;
    }
}

static void pull_at_startup(void)
{
    Git_Out o;
    char msg[256], before[64], after[64];

    emit('S', "Pulling...");
    head_of(before, sizeof before);
    git(&o, "pull", "--rebase", "--autostash", remote, branch, NULL);
    switch (classify(&o, msg, sizeof msg)) {
    case G_OK:
        head_of(after, sizeof after);
        emit(strcmp(before, after) ? 'U' : 'Y', NULL);
        break;
    case G_OFFLINE: emit('O', NULL); break;
    case G_CONFLICT: conflict_keep_remote(); break;
    default: emit('E', msg); break;
    }
}

static void do_startup(void)
{
    if (pull_startup)
        pull_at_startup();
    else
        emit('I', NULL);
    if (has_unsynced())
        emit('Q', NULL);
}

enum { RUN_STARTUP, RUN_PUSH, RUN_FULL };

static void worker_main(int mode)
{
    if (mode == RUN_STARTUP)
        do_startup();
    else
        do_push(mode == RUN_FULL);
    emit('Z', NULL);
}

#ifdef _WIN32

static DWORD WINAPI worker_thread_main(LPVOID mode)
{
    worker_main((int)(intptr_t)mode);
    return 0;
}

static int worker_start(int mode)
{
    worker_thread = CreateThread(NULL, 0, worker_thread_main, (LPVOID)(intptr_t)mode, 0, NULL);
    return worker_thread != NULL;
}

static void worker_reap(void)
{
    WaitForSingleObject(worker_thread, INFINITE);
    CloseHandle(worker_thread);
    worker_thread = NULL;
}

static void worker_abort(void)
{
    WaitForSingleObject(worker_thread, ABORT_WAIT_MS);
    CloseHandle(worker_thread);
    worker_thread = NULL;
}

static void flush_detached(void)
{
    char exe[MAX_PATH];
    String_Builder cmd = {0};
    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    char line_buf[2048];

    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    GetModuleFileNameA(NULL, exe, sizeof exe);
    snprintf(line_buf, sizeof line_buf, "\"%s\" --sync-flush \"%s\" \"%s\" \"%s\"", exe, dir, remote, branch);
    sb_set(&cmd, line_buf);
    if (CreateProcessA(NULL, cmd.data, NULL, NULL, FALSE, DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW,
                       NULL, NULL, &si, &pi)) {
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
    }
    sb_free(&cmd);
}

static void worker_platform_init(void)
{
    InitializeCriticalSection(&queue_lock);
    queue_event = CreateEventA(NULL, TRUE, FALSE, NULL);
}

#else

static void detach_stdio(void)
{
    int devnull = open("/dev/null", O_RDWR);

    setsid();
    dup2(devnull, 0);
    dup2(devnull, 1);
    dup2(devnull, 2);
}

static int worker_start(int mode)
{
    int p[2];
    pid_t pid;

    if (pipe(p))
        return 0;
    fflush(NULL);
    pid = fork();
    if (pid == 0) {
        close(p[0]);
        detach_stdio();
        event_fd = p[1];
        worker_main(mode);
        _exit(0);
    }
    close(p[1]);
    if (pid < 0) {
        close(p[0]);
        return 0;
    }
    worker_pid = pid;
    event_fd = p[0];
    return 1;
}

static void worker_reap(void)
{
    close(event_fd);
    event_fd = 0;
    waitpid(worker_pid, NULL, 0);
    worker_pid = 0;
}

static void worker_abort(void)
{
    kill(worker_pid, SIGTERM);
    worker_reap();
}

static void flush_detached(void)
{
    if (fork() != 0)
        return;
    detach_stdio();
    quiet = 1;
    do_push(0);
    _exit(0);
}

static void worker_platform_init(void)
{
}

#endif

static void spawn(int mode)
{
    if (worker_running || !worker_start(mode))
        return;
    worker_running = 1;
    line_len = 0;
    if (mode == RUN_STARTUP)
        return;
    pending_at = 0;
    force_next = 0;
}

void sync_init(const char *d, const Config *c, int enabled, Sync_Cb callback)
{
    char gitdir[1100];
    struct stat st;

    cb = callback;
    dir = xstrdup(d);
    remote = xstrdup(c->remote);
    branch = xstrdup(c->branch);
    debounce_ms = c->debounce * 1000;
    pull_startup = c->pull_on_startup;
    snprintf(gitdir, sizeof gitdir, "%s/.git", d);
    sync_enabled = enabled && stat(gitdir, &st) == 0;
    worker_platform_init();
    if (sync_enabled)
        spawn(RUN_STARTUP);
}

int sync_flush_main(const char *d, const char *r, const char *b)
{
    dir = xstrdup(d);
    remote = xstrdup(r);
    branch = xstrdup(b);
    quiet = 1;
    do_push(0);
    return 0;
}

static void worker_finished(void)
{
    worker_reap();
    worker_running = 0;
    sync_timers();
}

static void handle_line(const char *l)
{
    const char *m = l + 1;

    switch (l[0]) {
    case 'S': cb(SY_SYNCING, m); break;
    case 'I': cb(SY_IDLE, m); break;
    case 'Y':
        dirty = pending_at != 0;
        cb(SY_SYNCED, m);
        break;
    case 'U': cb(SY_UPDATED, m); break;
    case 'O':
        retry_at = now_ms() + OFFLINE_RETRY_MS;
        cb(SY_OFFLINE, m);
        break;
    case 'C': cb(SY_CONFLICT, m); break;
    case 'E': cb(SY_ERROR, m); break;
    case 'Q':
        pending_at = now_ms();
        dirty = 1;
        cb(SY_SYNCING, "Pending sync...");
        break;
    case 'Z': worker_finished(); break;
    }
}

static void feed_byte(char c)
{
    if (c != '\n') {
        if (line_len < sizeof line - 1)
            line[line_len++] = c;
        return;
    }
    line[line_len] = 0;
    if (line_len)
        handle_line(line);
    line_len = 0;
}

void sync_pump(void)
{
    char chunk[256];
    long n;

    if (!worker_running)
        return;
    n = worker_read(chunk, sizeof chunk);
    if (n < 0)
        worker_finished();
    for (long i = 0; i < n && worker_running; i++)
        feed_byte(chunk[i]);
}

void sync_request_push(void)
{
    if (!sync_enabled)
        return;
    dirty = 1;
    pending_at = now_ms();
    retry_at = 0;
    cb(SY_SYNCING, "Pending sync...");
}

void sync_force(void)
{
    if (!sync_enabled)
        return;
    force_next = 1;
    dirty = 1;
    if (!pending_at)
        pending_at = now_ms();
    sync_timers();
}

int sync_timeout_ms(void)
{
    int64_t due = 0, left;

    if (!sync_enabled || worker_running)
        return -1;
    if (pending_at)
        due = pending_at + debounce_ms;
    if (retry_at && (!due || retry_at < due))
        due = retry_at;
    if (!due)
        return -1;
    left = due - now_ms();
    return left < 0 ? 0 : left > 3600000 ? 3600000 : (int)left;
}

void sync_timers(void)
{
    int64_t t = now_ms();
    int full = force_next;
    int due = full || (pending_at && t >= pending_at + debounce_ms) || (retry_at && t >= retry_at);

    if (!sync_enabled || worker_running || !due)
        return;
    retry_at = 0;
    if (!pending_at)
        pending_at = t;
    spawn(full ? RUN_FULL : RUN_PUSH);
}

void sync_shutdown(void)
{
    if (!sync_enabled)
        return;
    if (worker_running)
        worker_abort();
    worker_running = 0;
    if (dirty)
        flush_detached();
}
