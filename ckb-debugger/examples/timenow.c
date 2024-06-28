#define CKB_C_STDLIB_PRINTF

#include "ckb_syscalls.h"
#include "stdio.h"
#include "stdlib.h"

uint64_t time() {
    return syscall(9001, 0, 0, 0, 0, 0, 0);
}

int main() {
    uint64_t tic = time();
    printf("%lu\n", tic);
    return 0;
}
