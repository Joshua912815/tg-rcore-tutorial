int main(void);
void _exit(int code);

void _start(void) {
    _exit(main());
}
