#include <ctype.h>
#include <fcntl.h>
#include <stdint.h>
#include <string.h>
#include <sys/time.h>
#include <unistd.h>

#include "../vendor/doomgeneric/doomgeneric.h"
#include "../vendor/doomgeneric/doomkeys.h"

#define SYS_FRAMEBUFFER_PRESENT 1041
#define SYS_INPUT_POLL 1042

#define KEYQUEUE_SIZE 32
#define ACTIVE_KEYS_MAX 16
#define KEY_PULSE_MS 120

static unsigned short s_KeyQueue[KEYQUEUE_SIZE];
static unsigned int s_KeyQueueWriteIndex = 0;
static unsigned int s_KeyQueueReadIndex = 0;

struct active_key {
    unsigned char doom_key;
    uint32_t release_at;
    int in_use;
};

static struct active_key s_ActiveKeys[ACTIVE_KEYS_MAX];

static long tg_syscall0(long n) {
    register long a0 asm("a0");
    register long a7 asm("a7") = n;
    asm volatile("ecall" : "=r"(a0) : "r"(a7) : "memory");
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

static unsigned char convert_to_doom_key(unsigned int key) {
    switch (key) {
        case '\n':
        case '\r':
            return KEY_ENTER;
        case 0x1b:
            return KEY_ESCAPE;
        case 'w':
            return KEY_UPARROW;
        case 's':
            return KEY_DOWNARROW;
        case 'a':
            return KEY_LEFTARROW;
        case 'd':
            return KEY_RIGHTARROW;
        case ' ':
        case 'f':
            return KEY_FIRE;
        case 'e':
            return KEY_USE;
        case 'q':
            return KEY_ESCAPE;
        case '=':
        case '+':
            return KEY_EQUALS;
        case '-':
            return KEY_MINUS;
        default:
            return (unsigned char) tolower((int) key);
    }
}

static void push_key_event(int pressed, unsigned char doom_key) {
    s_KeyQueue[s_KeyQueueWriteIndex] = (pressed << 8) | doom_key;
    s_KeyQueueWriteIndex = (s_KeyQueueWriteIndex + 1) % KEYQUEUE_SIZE;
}

static uint32_t tick_now_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (uint32_t) (tv.tv_sec * 1000 + tv.tv_usec / 1000);
}

static void release_expired_keys(void) {
    uint32_t now = tick_now_ms();
    int i;
    for (i = 0; i < ACTIVE_KEYS_MAX; ++i) {
        if (s_ActiveKeys[i].in_use && (int32_t) (now - s_ActiveKeys[i].release_at) >= 0) {
            push_key_event(0, s_ActiveKeys[i].doom_key);
            s_ActiveKeys[i].in_use = 0;
        }
    }
}

static void arm_key_release(unsigned char doom_key) {
    uint32_t deadline = tick_now_ms() + KEY_PULSE_MS;
    int i;
    int free_slot = -1;
    for (i = 0; i < ACTIVE_KEYS_MAX; ++i) {
        if (s_ActiveKeys[i].in_use && s_ActiveKeys[i].doom_key == doom_key) {
            s_ActiveKeys[i].release_at = deadline;
            return;
        }
        if (!s_ActiveKeys[i].in_use && free_slot < 0) {
            free_slot = i;
        }
    }
    if (free_slot >= 0) {
        s_ActiveKeys[free_slot].doom_key = doom_key;
        s_ActiveKeys[free_slot].release_at = deadline;
        s_ActiveKeys[free_slot].in_use = 1;
    }
}

static void queue_tap(unsigned char doom_key) {
    push_key_event(1, doom_key);
    arm_key_release(doom_key);
}

static void pump_input(void) {
    release_expired_keys();

    long key = tg_syscall0(SYS_INPUT_POLL);
    if (key < 0) {
        return;
    }

    if (key == 0x1b) {
        long b1 = tg_syscall0(SYS_INPUT_POLL);
        long b2 = tg_syscall0(SYS_INPUT_POLL);
        if (b1 == '[') {
            unsigned char doom_key = 0;
            switch (b2) {
                case 'A': doom_key = KEY_UPARROW; break;
                case 'B': doom_key = KEY_DOWNARROW; break;
                case 'C': doom_key = KEY_RIGHTARROW; break;
                case 'D': doom_key = KEY_LEFTARROW; break;
                default: break;
            }
            if (doom_key != 0) {
                queue_tap(doom_key);
                return;
            }
        }
    }

    {
        unsigned char doom_key = convert_to_doom_key((unsigned int) key);
        if (doom_key != 0) {
            queue_tap(doom_key);
        }
    }
}

void DG_Init(void) {
}

void DG_DrawFrame(void) {
    tg_syscall3(
        SYS_FRAMEBUFFER_PRESENT,
        (long) DG_ScreenBuffer,
        DOOMGENERIC_RESX,
        DOOMGENERIC_RESY
    );
    pump_input();
}

void DG_SleepMs(uint32_t ms) {
    uint32_t end = DG_GetTicksMs() + ms;
    while (DG_GetTicksMs() < end) {
        pump_input();
    }
}

uint32_t DG_GetTicksMs(void) {
    return tick_now_ms();
}

int DG_GetKey(int *pressed, unsigned char *doomKey) {
    if (s_KeyQueueReadIndex == s_KeyQueueWriteIndex) {
        return 0;
    }
    {
        unsigned short keyData = s_KeyQueue[s_KeyQueueReadIndex];
        s_KeyQueueReadIndex = (s_KeyQueueReadIndex + 1) % KEYQUEUE_SIZE;
        *pressed = keyData >> 8;
        *doomKey = keyData & 0xff;
        return 1;
    }
}

void DG_SetWindowTitle(const char *title) {
    (void) title;
}

static const char *pick_iwad(void) {
    const char *candidates[] = {
        "doom1.wad",
        "freedoom1.wad",
        NULL,
    };
    int i;
    for (i = 0; candidates[i] != NULL; ++i) {
        int fd = open(candidates[i], O_RDONLY);
        if (fd >= 0) {
            close(fd);
            return candidates[i];
        }
    }
    return "doom1.wad";
}

int main(void) {
    const char *iwad = pick_iwad();
    char *argv[] = {
        "doom",
        "-iwad",
        (char *) iwad,
        "-nosound",
        "-nomusic",
        NULL,
    };
    doomgeneric_Create(5, argv);
    for (;;) {
        doomgeneric_Tick();
    }
    return 0;
}
