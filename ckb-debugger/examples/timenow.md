riscv64-unknown-elf-gcc -fno-builtin-printf -nostdinc -nostdlib -nostartfiles -I ./ckb-c-stdlib/libc -I ./ckb-c-stdlib -g -Wl,-static -o timenow timenow.c
