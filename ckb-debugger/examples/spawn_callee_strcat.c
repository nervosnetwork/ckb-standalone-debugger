#include <stdint.h>
#include <string.h>

#include "ckb_syscalls.h"

char *strcat(char *restrict dest, const char *restrict src) {
    strcpy(dest + strlen(dest), src);
    return dest;
}

int main(int argc, char *argv[]) {
    int err = 0;
    char content[80];
    for (int i = 0; i < argc; i++) {
        strcat(content, argv[i]);
    }
    size_t content_size = (uint64_t)strlen(content);
    uint64_t fds[2] = {0};
    uint64_t length = 2;
    err = ckb_inherited_file_descriptors(fds, &length);
    if (err != 0) {
        return err;
    }
    if (length != 2) {
        return 1;
    }
    size_t content_size2 = content_size;
    printf("fds[CKB_STDOUT] = %lu", fds[1]);
    err = ckb_write(fds[1], content, &content_size);
    if (err != 0) {
        return err;
    }
    if (content_size2 != content_size) {
        return 1;
    }
    return 0;
}
