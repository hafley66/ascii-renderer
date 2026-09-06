// macOS libc-boundary counters for the dedicated renderer only.
// This observes calls/results, not kernel stacks or terminal display completion.
#include <errno.h>
#include <fcntl.h>
#include <mach/mach_time.h>
#include <poll.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#define FDS 128
typedef struct {
    _Atomic uint64_t calls, requested, bytes, again, errors, ns, max_ns, partial;
} Counts;
static Counts counts[3][FDS];
static _Atomic int enabled;
static int log_fd = -1;
static mach_timebase_info_data_t scale;
static uint64_t started;

static uint64_t clock_ns(void) {
    return mach_absolute_time() * scale.numer / scale.denom;
}
static void add(int op, int fd, size_t requested, ssize_t result, int error, uint64_t elapsed) {
    if (fd < 0 || fd >= FDS || fd == log_fd) return;
    Counts *c = &counts[op][fd];
    atomic_fetch_add(&c->calls, 1);
    atomic_fetch_add(&c->requested, requested);
    atomic_fetch_add(&c->ns, elapsed);
    uint64_t max = atomic_load(&c->max_ns);
    while (elapsed > max && !atomic_compare_exchange_weak(&c->max_ns, &max, elapsed)) {}
    if (result >= 0) {
        atomic_fetch_add(&c->bytes, result);
        if ((size_t)result < requested) atomic_fetch_add(&c->partial, 1);
    } else if (error == EAGAIN) atomic_fetch_add(&c->again, 1);
    else atomic_fetch_add(&c->errors, 1);
}

static ssize_t probe_write(int fd, const void *data, size_t size) {
    if (!atomic_load(&enabled)) return write(fd, data, size);
    uint64_t t = clock_ns();
    ssize_t n = write(fd, data, size); int e = errno;
    add(1, fd, size, n, e, clock_ns() - t); errno = e; return n;
}
static ssize_t probe_read(int fd, void *data, size_t size) {
    if (!atomic_load(&enabled)) return read(fd, data, size);
    uint64_t t = clock_ns();
    ssize_t n = read(fd, data, size); int e = errno;
    add(0, fd, size, n, e, clock_ns() - t); errno = e; return n;
}
static int probe_poll(struct pollfd *fds, nfds_t n, int timeout) {
    if (!atomic_load(&enabled)) return poll(fds, n, timeout);
    uint64_t t = clock_ns();
    int result = poll(fds, n, timeout); int e = errno;
    add(2, n == 1 ? fds[0].fd : 127, 0, result, e, clock_ns() - t);
    errno = e; return result;
}

#define INTERPOSE(replacement, original) \
 __attribute__((used)) static struct { const void *new_fn; const void *old_fn; } \
 interpose_##original __attribute__((section("__DATA,__interpose"))) = \
 { (const void *)&replacement, (const void *)&original }
INTERPOSE(probe_write, write);
INTERPOSE(probe_read, read);
INTERPOSE(probe_poll, poll);

static void snapshot(void) {
    struct timespec now; clock_gettime(CLOCK_REALTIME, &now);
    uint64_t epoch = now.tv_sec * 1000ULL + now.tv_nsec / 1000000;
    const char *ops[] = {"read", "write", "poll"};
    for (int op = 0; op < 3; ++op) for (int fd = 0; fd < FDS; ++fd) {
        Counts *c = &counts[op][fd];
        if (!atomic_load(&c->calls)) continue;
        char line[1024]; struct stat st;
        const char *kind = fstat(fd, &st) ? "closed" : S_ISFIFO(st.st_mode) ? "pipe" :
            S_ISCHR(st.st_mode) ? "char" : S_ISREG(st.st_mode) ? "file" : "other";
        int n = snprintf(line, sizeof line,
            "{\"ts_ms\":%llu,\"pid\":%d,\"elapsed_ns\":%llu,\"op\":\"%s\",\"fd\":%d,"
            "\"fd_kind_now\":\"%s\",\"calls\":%llu,\"requested\":%llu,\"bytes\":%llu,"
            "\"eagain\":%llu,\"errors\":%llu,\"partial\":%llu,\"call_ns\":%llu,\"max_ns\":%llu}\n",
            epoch, getpid(), clock_ns()-started, ops[op], fd, kind,
            atomic_load(&c->calls), atomic_load(&c->requested), atomic_load(&c->bytes),
            atomic_load(&c->again), atomic_load(&c->errors), atomic_load(&c->partial),
            atomic_load(&c->ns), atomic_load(&c->max_ns));
        write(log_fd, line, n);
    }
}
static void *monitor(void *unused) {
    while (1) { sleep(1); snapshot(); }
    return NULL;
}
__attribute__((constructor)) static void setup(void) {
    const char *dir = getenv("ASCII_SYSCALL_DIR");
    if (!dir || strcmp(getprogname(), "ascii-renderer")) return;
    char path[2048]; snprintf(path, sizeof path, "%s/io-%d.ndjson", dir, getpid());
    log_fd = open(path, O_WRONLY|O_CREAT|O_APPEND|O_CLOEXEC, 0600);
    if (log_fd < 0) return;
    mach_timebase_info(&scale); started = clock_ns();
    atomic_store(&enabled, 1);
    pthread_t thread; if (!pthread_create(&thread, NULL, monitor, NULL)) pthread_detach(thread);
}
__attribute__((destructor)) static void finish(void) {
    if (atomic_load(&enabled)) snapshot();
}
