FROM gcc:9

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        binutils-riscv64-unknown-elf \
        gcc-riscv64-unknown-elf \
        picolibc-riscv64-unknown-elf \
    && rm -rf /var/lib/apt/lists/*
