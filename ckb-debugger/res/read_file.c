#define CKB_C_STDLIB_PRINTF
#define CKB_C_STDLIB_PRINTF_BUFFER_SIZE 1024 * 16

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "ckb_syscalls.h"

uint64_t now() { return syscall(9001, 0, 0, 0, 0, 0, 0); }

int read_file(char* buf, int size) {
    int ret = syscall(9000, buf, size, 0, 0, 0, 0);
    return ret;
}

uint64_t random() { return syscall(9002, 0, 0, 0, 0, 0, 0); }

int main() {
    char buf[1024 * 16] = {0};
    uint64_t start = now();
    int read_size = read_file(buf, sizeof(buf));
    printf("read size = %d", read_size);
    printf("------ content --------");
    printf("%s", buf);

    int duration = now() - start;
    printf("duration = %lld milli-second", duration / 1000 / 1000);
    printf("generate a random number: %llu", random());

    return 0;
}
