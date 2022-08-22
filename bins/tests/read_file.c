#define CKB_C_STDLIB_PRINTF
#define CKB_C_STDLIB_PRINTF_BUFFER_SIZE 1024*16

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "ckb_syscalls.h"

typedef unsigned __int128 uint128_t;

uint128_t now() {
    uint128_t now = 0;
    syscall(9001, &now, 0, 0, 0, 0, 0);
    return now;
}

int read_file(char* buf, int size) {
    int ret = syscall(9000, buf, size, 0, 0, 0, 0);
    return ret;
}

uint64_t random() {
    uint64_t r = 0;
    syscall(9002, &r, 0, 0, 0, 0, 0);
    return r;
}

int main() {
    char buf[1024*16] = {0};
    uint128_t start = now();

    int read_size = read_file(buf, sizeof(buf));
    printf("read size = %d", read_size);
    printf("------ content --------");
    printf("%s", buf);

    int duration = now() - start;
    printf("duration = %lld milli-second", duration/1000/1000);
    printf("generate a random number: %llu", random());

    return 0;
}
