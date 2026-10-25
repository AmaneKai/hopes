#include "hopes.h"

#ifdef _WIN32

#define CONSOLE_GRACE_MS 4000

static HANDLE wake_event;

static BOOL WINAPI on_console_event(DWORD type)
{
    if (type == CTRL_C_EVENT)
        return TRUE;
    SetEvent(wake_event);
    Sleep(CONSOLE_GRACE_MS);
    return TRUE;
}

void plat_init(void)
{
    SetEnvironmentVariableA("GIT_TERMINAL_PROMPT", "0");
    wake_event = CreateEventA(NULL, TRUE, FALSE, NULL);
    SetConsoleCtrlHandler(on_console_event, TRUE);
}

int64_t now_ms(void)
{
    return (int64_t)GetTickCount64();
}

char *plat_base_dir(int config)
{
    const char *env = getenv(config ? "APPDATA" : "LOCALAPPDATA");

    return xstrdup(env ? env : ".");
}

char *plat_state_dir(void)
{
    return plat_base_dir(0);
}

void plat_mkdir(const char *path)
{
    _mkdir(path);
}

int plat_replace(const char *from, const char *to)
{
    return MoveFileExA(from, to, MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) ? 0 : -1;
}

int plat_fsync(int fd)
{
    return _commit(fd);
}

void plat_localtime(time_t t, struct tm *out)
{
    localtime_s(out, &t);
}

int plat_wait(int timeout_ms, intptr_t sync_handle)
{
    HANDLE handles[3] = {GetStdHandle(STD_INPUT_HANDLE), wake_event, (HANDLE)sync_handle};
    DWORD count = sync_handle ? 3 : 2;
    DWORD ms = timeout_ms < 0 ? INFINITE : (DWORD)timeout_ms;
    DWORD r = WaitForMultipleObjects(count, handles, FALSE, ms);

    if (r == WAIT_OBJECT_0)
        return PLAT_INPUT;
    if (r == WAIT_OBJECT_0 + 1)
        return PLAT_HUP;
    if (r == WAIT_OBJECT_0 + 2)
        return PLAT_SYNC;
    return r == WAIT_FAILED ? PLAT_HUP : 0;
}

static void append_slashes(String_Builder *cmd, size_t n)
{
    while (n--)
        sb_putc(cmd, '\\');
}

static void quote_arg(String_Builder *cmd, const char *arg)
{
    int plain = *arg && !strpbrk(arg, " \t\n\v\"");

    if (cmd->count)
        sb_putc(cmd, ' ');
    if (plain) {
        sb_append(cmd, arg, strlen(arg));
        return;
    }
    sb_putc(cmd, '"');
    for (;; arg++) {
        size_t slashes = 0;

        while (*arg == '\\') {
            arg++;
            slashes++;
        }
        if (!*arg) {
            append_slashes(cmd, slashes * 2);
            break;
        }
        append_slashes(cmd, *arg == '"' ? slashes * 2 + 1 : slashes);
        sb_putc(cmd, *arg);
    }
    sb_putc(cmd, '"');
}

static HANDLE open_inheritable(const char *path, DWORD access, DWORD disposition)
{
    SECURITY_ATTRIBUTES sa = {sizeof sa, NULL, TRUE};

    return CreateFileA(path, access, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, &sa,
                       disposition, FILE_ATTRIBUTE_NORMAL, NULL);
}

typedef struct {
    HANDLE pipe;
    String_Builder *sb;
    size_t limit;
} Drain;

static DWORD WINAPI drain_main(LPVOID arg)
{
    Drain *d = arg;
    char chunk[4096];
    DWORD n;

    while (ReadFile(d->pipe, chunk, sizeof chunk, &n, NULL) && n) {
        size_t room = d->sb && d->sb->count < d->limit ? d->limit - d->sb->count : 0;

        if (room)
            sb_append(d->sb, chunk, n < room ? n : room);
    }
    return 0;
}

typedef struct {
    HANDLE pipe;
    const char *data;
    size_t len;
} Feed;

static DWORD WINAPI feed_main(LPVOID arg)
{
    Feed *f = arg;
    size_t done = 0;
    DWORD n;

    while (done < f->len && WriteFile(f->pipe, f->data + done, (DWORD)(f->len - done), &n, NULL) && n)
        done += n;
    CloseHandle(f->pipe);
    return 0;
}

static int make_pipe(HANDLE *parent, HANDLE *child, int parent_reads)
{
    SECURITY_ATTRIBUTES sa = {sizeof sa, NULL, TRUE};
    HANDLE read_end, write_end;

    if (!CreatePipe(&read_end, &write_end, &sa, 0))
        return -1;
    *parent = parent_reads ? read_end : write_end;
    *child = parent_reads ? write_end : read_end;
    SetHandleInformation(*parent, HANDLE_FLAG_INHERIT, 0);
    return 0;
}

static int wait_process(PROCESS_INFORMATION *pi, HANDLE job, int timeout_ms)
{
    DWORD code = 0;
    int rc;

    if (WaitForSingleObject(pi->hProcess, timeout_ms < 0 ? INFINITE : (DWORD)timeout_ms) == WAIT_TIMEOUT) {
        if (job)
            TerminateJobObject(job, 1);
        else
            TerminateProcess(pi->hProcess, 1);
        WaitForSingleObject(pi->hProcess, 2000);
        rc = -2;
    } else {
        GetExitCodeProcess(pi->hProcess, &code);
        rc = (int)code;
    }
    CloseHandle(pi->hProcess);
    CloseHandle(pi->hThread);
    return rc;
}

static HANDLE new_job(void)
{
    return CreateJobObjectA(NULL, NULL);
}

static int start_process(String_Builder *cmd, const char *dir, STARTUPINFOA *si, PROCESS_INFORMATION *pi, HANDLE job)
{
    if (!CreateProcessA(NULL, cmd->data, NULL, NULL, TRUE, CREATE_NO_WINDOW | CREATE_SUSPENDED, NULL, dir, si, pi))
        return -1;
    if (job)
        AssignProcessToJobObject(job, pi->hProcess);
    ResumeThread(pi->hThread);
    return 0;
}

int plat_run(char *const argv[], const char *dir, const String_Builder *input, String_Builder *out,
             String_Builder *err, size_t limit, int timeout_ms)
{
    String_Builder cmd = {0};
    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    HANDLE in_parent = NULL, in_child = NULL, out_parent = NULL, out_child = NULL, err_parent = NULL, err_child = NULL;
    HANDLE threads[3] = {0}, job = new_job();
    Drain drains[2] = {{0, out, limit}, {0, err, limit}};
    Feed feed = {0};
    int rc = -1, count = 0;

    if (make_pipe(&out_parent, &out_child, 1) || make_pipe(&err_parent, &err_child, 1))
        goto done;
    if (input && make_pipe(&in_parent, &in_child, 0))
        goto done;
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = input ? in_child : open_inheritable("NUL", GENERIC_READ, OPEN_EXISTING);
    si.hStdOutput = out_child;
    si.hStdError = err_child;
    for (int i = 0; argv[i]; i++)
        quote_arg(&cmd, argv[i]);

    if (!start_process(&cmd, dir, &si, &pi, job)) {
        drains[0].pipe = out_parent;
        drains[1].pipe = err_parent;
        threads[count++] = CreateThread(NULL, 0, drain_main, &drains[0], 0, NULL);
        threads[count++] = CreateThread(NULL, 0, drain_main, &drains[1], 0, NULL);
        if (input) {
            feed = (Feed){in_parent, sb_cstr(input), input->count};
            threads[count++] = CreateThread(NULL, 0, feed_main, &feed, 0, NULL);
            in_parent = NULL;
        }
        CloseHandle(si.hStdOutput);
        CloseHandle(si.hStdError);
        out_child = err_child = NULL;
        rc = wait_process(&pi, job, timeout_ms);
        WaitForMultipleObjects((DWORD)count, threads, TRUE, 5000);
    }
    if (!input && si.hStdInput)
        CloseHandle(si.hStdInput);
done:
    for (int i = 0; i < count; i++)
        CloseHandle(threads[i]);
    for (HANDLE *h = (HANDLE[]){in_parent, in_child, out_parent, out_child, err_parent, err_child}; 0;)
        (void)h;
    if (in_parent)
        CloseHandle(in_parent);
    if (in_child)
        CloseHandle(in_child);
    if (out_parent)
        CloseHandle(out_parent);
    if (out_child)
        CloseHandle(out_child);
    if (err_parent)
        CloseHandle(err_parent);
    if (err_child)
        CloseHandle(err_child);
    if (job)
        CloseHandle(job);
    sb_free(&cmd);
    return rc;
}

#else

#define PIPE_STEP 4096

void plat_init(void)
{
    signal(SIGPIPE, SIG_IGN);
    setenv("GIT_TERMINAL_PROMPT", "0", 1);
}

int64_t now_ms(void)
{
    struct timespec ts;

    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

#ifdef __APPLE__
char *plat_base_dir(int config)
{
    const char *home = getenv("HOME");
    size_t n = strlen(home ? home : ".") + 32;
    char *p = xmalloc(n);

    (void)config;
    snprintf(p, n, "%s/Library/Application Support", home ? home : ".");
    return p;
}

char *plat_state_dir(void)
{
    return plat_base_dir(0);
}
#else
char *plat_base_dir(int config)
{
    const char *env = getenv(config ? "XDG_CONFIG_HOME" : "XDG_DATA_HOME");
    const char *home = getenv("HOME");
    size_t n = strlen(home ? home : ".") + 24;
    char *p;

    if (env && env[0] == '/')
        return xstrdup(env);
    p = xmalloc(n);
    snprintf(p, n, "%s/%s", home ? home : ".", config ? ".config" : ".local/share");
    return p;
}

char *plat_state_dir(void)
{
    const char *env = getenv("XDG_STATE_HOME");
    const char *home = getenv("HOME");
    size_t n = strlen(home ? home : ".") + 24;
    char *p;

    if (env && env[0] == '/')
        return xstrdup(env);
    p = xmalloc(n);
    snprintf(p, n, "%s/.local/state", home ? home : ".");
    return p;
}
#endif

void plat_mkdir(const char *path)
{
    mkdir(path, 0755);
}

int plat_replace(const char *from, const char *to)
{
    return rename(from, to);
}

int plat_fsync(int fd)
{
    return fsync(fd);
}

void plat_localtime(time_t t, struct tm *out)
{
    localtime_r(&t, out);
}

int plat_wait(int timeout_ms, intptr_t sync_handle)
{
    struct pollfd fds[2] = {{0, POLLIN, 0}, {(int)sync_handle, POLLIN, 0}};
    int rc = poll(fds, sync_handle >= 0 ? 2 : 1, timeout_ms), events = 0;

    if (rc <= 0)
        return 0;
    if (fds[0].revents & POLLIN)
        events |= PLAT_INPUT;
    else if (fds[0].revents & (POLLHUP | POLLERR))
        events |= PLAT_HUP;
    if (sync_handle >= 0 && fds[1].revents)
        events |= PLAT_SYNC;
    return events;
}

static void close_pair(const int fds[2])
{
    for (int i = 0; i < 2; i++)
        if (fds[i] >= 0)
            close(fds[i]);
}

static void run_child(char *const argv[], const char *dir, const int in[2], const int out[2],
                      const int err[2])
{
    int devnull = open("/dev/null", O_RDWR);

    dup2(in[0] >= 0 ? in[0] : devnull, 0);
    dup2(out[1] >= 0 ? out[1] : devnull, 1);
    dup2(err[1] >= 0 ? err[1] : devnull, 2);
    close_pair(in);
    close_pair(out);
    close_pair(err);
    close(devnull);
    if (dir && chdir(dir))
        _exit(126);
    execvp(argv[0], argv);
    _exit(127);
}

static int service(struct pollfd *p, String_Builder *sb, size_t limit)
{
    char chunk[4096];
    ssize_t n;

    if (p->fd < 0 || !p->revents)
        return 0;
    n = read(p->fd, chunk, sizeof chunk);
    if (n < 0 && errno == EINTR)
        return 0;
    if (n <= 0) {
        p->fd = -1;
        return 1;
    }
    if (sb && sb->count < limit)
        sb_append(sb, chunk, (size_t)n < limit - sb->count ? (size_t)n : limit - sb->count);
    return 0;
}

static void feed_child(struct pollfd *p, const String_Builder *input, size_t *sent)
{
    size_t left = input->count - *sent, step = left < PIPE_STEP ? left : PIPE_STEP;
    ssize_t w = 0;

    if (p->fd < 0 || !p->revents)
        return;
    if (step)
        w = write(p->fd, sb_cstr(input) + *sent, step);
    if (w < 0 && errno == EINTR)
        return;
    if (w > 0)
        *sent += (size_t)w;
    if (w < 0 || !step || *sent == input->count) {
        close(p->fd);
        p->fd = -1;
    }
}

static int pump(pid_t pid, int in_fd, const String_Builder *input, int out_fd, int err_fd, String_Builder *out,
                String_Builder *err, size_t limit, int timeout_ms)
{
    struct pollfd fds[3] = {{out_fd, POLLIN, 0}, {err_fd, POLLIN, 0}, {in_fd, POLLOUT, 0}};
    int64_t deadline = now_ms() + timeout_ms;
    size_t sent = 0;
    int timed_out = 0;

    while (fds[0].fd >= 0 || fds[1].fd >= 0) {
        int64_t left = deadline - now_ms();

        if (timeout_ms >= 0 && left <= 0) {
            kill(pid, SIGKILL);
            timed_out = 1;
            break;
        }
        if (poll(fds, 3, timeout_ms < 0 ? -1 : (int)left) < 0 && errno != EINTR)
            break;
        service(&fds[0], out, limit);
        service(&fds[1], err, limit);
        if (input)
            feed_child(&fds[2], input, &sent);
    }
    if (fds[2].fd >= 0)
        close(fds[2].fd);
    return timed_out;
}

int plat_run(char *const argv[], const char *dir, const String_Builder *input, String_Builder *out,
             String_Builder *err, size_t limit, int timeout_ms)
{
    int in[2] = {-1, -1}, po[2], pe[2], status = 0;
    int timed_out;
    pid_t pid;

    if (pipe(po) || pipe(pe) || (input && pipe(in)))
        return -1;
    pid = fork();
    if (pid < 0)
        return -1;
    if (pid == 0)
        run_child(argv, dir, in, po, pe);

    close(po[1]);
    close(pe[1]);
    if (input)
        close(in[0]);
    timed_out = pump(pid, input ? in[1] : -1, input, po[0], pe[0], out, err, limit, timeout_ms);
    close(po[0]);
    close(pe[0]);
    while (waitpid(pid, &status, 0) < 0 && errno == EINTR)
        ;
    if (timed_out)
        return -2;
    return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
}

#endif
