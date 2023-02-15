#define CKB_C_STDLIB_PRINTF
#define CKB_C_STDLIB_PRINTF_BUFFER_SIZE 1024 * 16

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "ckb_syscalls.h"

void* fopen(const char* path, const char* mode) {
    return (void*)syscall(9003, path, mode, 0, 0, 0, 0);
}

void* freopen(const char* path, const char* mode, void* stream) {
    return (void*)syscall(9004, path, mode, stream, 0, 0, 0);
}

uint64_t fread(void* ptr, size_t size, size_t nitems, void* stream) {
    return syscall(9005, ptr, size, nitems, stream, 0, 0);
}

int feof(void* stream) { return syscall(9006, stream, 0, 0, 0, 0, 0); }

int ferror(void* stream) { return syscall(9007, stream, 0, 0, 0, 0, 0); }

int fgetc(void* stream) { return syscall(9008, stream, 0, 0, 0, 0, 0); }

int fclose(void* stream) { return syscall(9009, stream, 0, 0, 0, 0, 0); }

long ftell(void* stream) { return syscall(9010, stream, 0, 0, 0, 0, 0); }

int fseek(void* stream, long offset, int whence) {
    return syscall(9011, stream, offset, whence, 0, 0, 0);
}

int main() {
    printf("Entering main");
    void* stream = fopen("fib.c", "r");
    if (!stream) {
        printf("Testing fopen failed");
        return -1;
    }

    char content[1024] = {0};
    int error = ferror(stream);
    if (error) {
        printf("Testing ferror failed");
        return -1;
    }
    int count = fread(content, 1, sizeof(content), stream);
    if (count < 2) {
        printf("Testing fread failed");
        return -1;
    }
    int eof = feof(stream);
    if (!eof) {
        printf("Testing feof failed");
        return -1;
    }
    stream = freopen("fib.c", "r", stream);
    if (!stream) {
        printf("Testing freopen failed");
        return -1;
    }
    int ch = fgetc(stream);
    if (ch == 0) {
        printf("Testing fgetc failed");
        return -1;
    }
    int pos = ftell(stream);
    if (pos == 0) {
        printf("Testing ftell failed");
        return -1;
    }
    int code = fseek(stream, 0, 0);
    if (code != 0) {
        printf("Testing fseek failed");
        return -1;
    }
    code = fclose(stream);
    if (code != 0) {
        printf("Testing fclose failed");
        return -1;
    }

    printf("--------content of file----------");
    printf("%s", content);
    printf("------------------");
    return 0;
}
