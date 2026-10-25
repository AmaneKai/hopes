#include "hopes.h"

#ifdef _WIN32

void plat_init(void)
{
    SetEnvironmentVariableA("GIT_TERMINAL_PROMPT", "0");
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
    HANDLE handles[2] = {GetStdHandle(STD_INPUT_HANDLE), (HANDLE)sync_handle};
    DWORD count = sync_handle ? 2 : 1;
    DWORD ms = timeout_ms < 0 ? INFINITE : (DWORD)timeout_ms;
    DWORD r = WaitForMultipleObjects(count, handles, FALSE, ms);

    if (r == WAIT_OBJECT_0)
        return PLAT_INPUT;
    if (r == WAIT_OBJECT_0 + 1)
        return PLAT_SYNC;
    return r == WAIT_FAILED ? PLAT_HUP : 0;
}

static void quote_arg(String_Builder *cmd, const char *arg)
{
    int plain = *arg && !strpbrk(arg, " \t\"");

    if (cmd->count)
        sb_putc(cmd, ' ');
    if (plain) {
        sb_append(cmd, arg, strlen(arg));
        return;
    }
    sb_putc(cmd, '"');
    for (; *arg; arg++) {
        if (*arg == '"')
            sb_putc(cmd, '\\');
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

static void slurp(const char *path, String_Builder *sb, size_t limit)
{
    char chunk[512];
    size_t n;
    FILE *f = fopen(path, "rb");

    if (!f)
        return;
    while (sb && sb->count < limit && (n = fread(chunk, 1, sizeof chunk, f)) > 0)
        sb_append(sb, chunk, n < limit - sb->count ? n : limit - sb->count);
    fclose(f);
}

static int wait_process(PROCESS_INFORMATION *pi, int timeout_ms)
{
    DWORD code = 0;

    if (WaitForSingleObject(pi->hProcess, timeout_ms < 0 ? INFINITE : (DWORD)timeout_ms) == WAIT_TIMEOUT) {
        TerminateProcess(pi->hProcess, 1);
        WaitForSingleObject(pi->hProcess, 1000);
        CloseHandle(pi->hProcess);
        CloseHandle(pi->hThread);
        return -2;
    }
    GetExitCodeProcess(pi->hProcess, &code);
    CloseHandle(pi->hProcess);
    CloseHandle(pi->hThread);
    return (int)code;
}

int plat_run(char *const argv[], const char *dir, const String_Builder *input, String_Builder *out,
             String_Builder *err, size_t limit, int timeout_ms)
{
    static volatile LONG serial;
    char base[MAX_PATH + 64], in_path[MAX_PATH + 80], out_path[MAX_PATH + 80], err_path[MAX_PATH + 80];
    String_Builder cmd = {0};
    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    int rc = -1;

    GetTempPathA(MAX_PATH, base);
    snprintf(base + strlen(base), 64, "hopes-%lu-%ld", GetCurrentProcessId(), InterlockedIncrement(&serial));
    snprintf(in_path, sizeof in_path, "%s.in", base);
    snprintf(out_path, sizeof out_path, "%s.out", base);
    snprintf(err_path, sizeof err_path, "%s.err", base);

    if (input) {
        FILE *f = fopen(in_path, "wb");

        if (f) {
            fwrite(sb_cstr(input), 1, input->count, f);
            fclose(f);
        }
    }
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    si.dwFlags = STARTF_USESTDHANDLES;
    si.hStdInput = open_inheritable(input ? in_path : "NUL", GENERIC_READ, OPEN_EXISTING);
    si.hStdOutput = open_inheritable(out_path, GENERIC_WRITE, CREATE_ALWAYS);
    si.hStdError = open_inheritable(err_path, GENERIC_WRITE, CREATE_ALWAYS);
    for (int i = 0; argv[i]; i++)
        quote_arg(&cmd, argv[i]);

    if (CreateProcessA(NULL, cmd.data, NULL, NULL, TRUE, CREATE_NO_WINDOW, NULL, dir, &si, &pi))
        rc = wait_process(&pi, timeout_ms);
    CloseHandle(si.hStdInput);
    CloseHandle(si.hStdOutput);
    CloseHandle(si.hStdError);

    slurp(out_path, out, limit);
    slurp(err_path, err, limit);
    remove(in_path);
    remove(out_path);
    remove(err_path);
    sb_free(&cmd);
    return rc;
}

#else

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
    char chunk[512];
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

static int pump_outputs(pid_t pid, int out_fd, int err_fd, String_Builder *out, String_Builder *err,
                         size_t limit, int timeout_ms)
{
    struct pollfd fds[2] = {{out_fd, POLLIN, 0}, {err_fd, POLLIN, 0}};
    int64_t deadline = now_ms() + timeout_ms;
    int open_fds = 2;

    while (open_fds) {
        int64_t left = deadline - now_ms();

        if (timeout_ms >= 0 && left <= 0) {
            kill(pid, SIGKILL);
            return 1;
        }
        if (poll(fds, 2, timeout_ms < 0 ? -1 : (int)left) < 0 && errno != EINTR)
            break;
        open_fds -= service(&fds[0], out, limit) + service(&fds[1], err, limit);
    }
    return 0;
}

static void feed_stdin(int fd, const String_Builder *input)
{
    const char *p = sb_cstr(input);
    size_t left = input->count;

    while (left) {
        ssize_t w = write(fd, p, left);

        if (w < 0 && errno == EINTR)
            continue;
        if (w <= 0)
            break;
        p += w;
        left -= (size_t)w;
    }
    close(fd);
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
    if (input) {
        close(in[0]);
        feed_stdin(in[1], input);
    }
    timed_out = pump_outputs(pid, po[0], pe[0], out, err, limit, timeout_ms);
    close(po[0]);
    close(pe[0]);
    while (waitpid(pid, &status, 0) < 0 && errno == EINTR)
        ;
    if (timed_out)
        return -2;
    return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
}

#endif
