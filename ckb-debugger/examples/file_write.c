#define CKB_C_STDLIB_PRINTF
#define CKB_C_STDLIB_PRINTF_BUFFER_SIZE 1024 * 16

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "ckb_syscalls.h"

int file_write(const char* path, void *ptr, size_t size) {
    return syscall(9013, path, ptr, size, 0, 0, 0);
}

int main() {
    char *data = "Hello World!\n";
    size_t size = 13;
    file_write("file_write_foo.txt", data, size);
    return 0;
}
