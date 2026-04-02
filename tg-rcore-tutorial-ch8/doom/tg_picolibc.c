#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <unistd.h>

#define SYS_OPENAT 56
#define SYS_CLOSE 57
#define SYS_LSEEK 62
#define SYS_READ 63
#define SYS_WRITE 64
#define SYS_EXIT 93
#define SYS_CLOCK_GETTIME 113

static long tg_syscall0(long n) {
    register long a0 asm("a0");
    register long a7 asm("a7") = n;
    asm volatile("ecall" : "=r"(a0) : "r"(a7) : "memory");
    return a0;
}

static long tg_syscall1(long n, long a0v) {
    register long a0 asm("a0") = a0v;
    register long a7 asm("a7") = n;
    asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");
    return a0;
}

static long tg_syscall2(long n, long a0v, long a1v) {
    register long a0 asm("a0") = a0v;
    register long a1 asm("a1") = a1v;
    register long a7 asm("a7") = n;
    asm volatile("ecall" : "+r"(a0) : "r"(a1), "r"(a7) : "memory");
    return a0;
}

static long tg_syscall3(long n, long a0v, long a1v, long a2v) {
    register long a0 asm("a0") = a0v;
    register long a1 asm("a1") = a1v;
    register long a2 asm("a2") = a2v;
    register long a7 asm("a7") = n;
    asm volatile("ecall" : "+r"(a0) : "r"(a1), "r"(a2), "r"(a7) : "memory");
    return a0;
}

static int tg_stdout_put(char c, FILE *stream) {
    (void) stream;
    if (tg_syscall3(SYS_WRITE, STDOUT_FILENO, (long) &c, 1) < 0) {
        return EOF;
    }
    return 0;
}

static int tg_stdin_get(FILE *stream) {
    char c = 0;
    (void) stream;
    if (tg_syscall3(SYS_READ, STDIN_FILENO, (long) &c, 1) <= 0) {
        return EOF;
    }
    return (unsigned char) c;
}

static int tg_flush(FILE *stream) {
    (void) stream;
    return 0;
}

static FILE tg_stdin = FDEV_SETUP_STREAM(NULL, tg_stdin_get, tg_flush, _FDEV_SETUP_READ);
static FILE tg_stdout = FDEV_SETUP_STREAM(tg_stdout_put, NULL, tg_flush, _FDEV_SETUP_WRITE);
static FILE tg_stderr = FDEV_SETUP_STREAM(tg_stdout_put, NULL, tg_flush, _FDEV_SETUP_WRITE);

struct __file *const __iob[] = {
    &tg_stdin,
    &tg_stdout,
    &tg_stderr,
};

void _exit(int code) {
    tg_syscall1(SYS_EXIT, code);
    while (1) {}
}

int _open(const char *path, int flags, ...) {
    long ret = tg_syscall2(SYS_OPENAT, (long) path, flags);
    if (ret < 0) {
        return -1;
    }
    return (int) ret;
}

int open(const char *path, int flags, ...) {
    return _open(path, flags);
}

int _close(int fd) {
    long ret = tg_syscall1(SYS_CLOSE, fd);
    if (ret < 0) {
        return -1;
    }
    return (int) ret;
}

int close(int fd) {
    return _close(fd);
}

ssize_t _read(int fd, void *buf, size_t count) {
    long ret = tg_syscall3(SYS_READ, fd, (long) buf, count);
    if (ret < 0) {
        return -1;
    }
    return (ssize_t) ret;
}

ssize_t read(int fd, void *buf, size_t count) {
    return _read(fd, buf, count);
}

ssize_t _write(int fd, const void *buf, size_t count) {
    long ret = tg_syscall3(SYS_WRITE, fd, (long) buf, count);
    if (ret < 0) {
        return -1;
    }
    return (ssize_t) ret;
}

ssize_t write(int fd, const void *buf, size_t count) {
    return _write(fd, buf, count);
}

off_t _lseek(int fd, off_t offset, int whence) {
    long ret = tg_syscall3(SYS_LSEEK, fd, offset, whence);
    if (ret < 0) {
        return -1;
    }
    return (off_t) ret;
}

off_t lseek(int fd, off_t offset, int whence) {
    return _lseek(fd, offset, whence);
}

int _fstat(int fd, struct stat *st) {
    (void) fd;
    if (st == NULL) {
        return -1;
    }
    st->st_mode = S_IFREG;
    st->st_nlink = 1;
    st->st_blksize = 512;
    return 0;
}

int _isatty(int fd) {
    return fd <= STDERR_FILENO;
}

int _getpid(void) {
    return 1;
}

int _kill(int pid, int sig) {
    (void) pid;
    (void) sig;
    return -1;
}

int _gettimeofday(struct timeval *tv, void *tz) {
    struct {
        long tv_sec;
        long tv_nsec;
    } ts;
    long ret;
    (void) tz;
    if (tv == NULL) {
        return -1;
    }
    ret = tg_syscall2(SYS_CLOCK_GETTIME, 1, (long) &ts);
    if (ret < 0) {
        return -1;
    }
    tv->tv_sec = ts.tv_sec;
    tv->tv_usec = ts.tv_nsec / 1000;
    return 0;
}

int gettimeofday(struct timeval *tv, void *tz) {
    return _gettimeofday(tv, tz);
}

int mkdir(const char *path, mode_t mode) {
    (void) path;
    (void) mode;
    return -1;
}

int remove(const char *path) {
    (void) path;
    return -1;
}

int rename(const char *oldpath, const char *newpath) {
    (void) oldpath;
    (void) newpath;
    return -1;
}
