#include <stdint.h>
#include <string.h>

#include "ckb_syscalls.h"

// Mimic stdio fds on linux
int create_std_fds(uint64_t* fds, uint64_t* inherited_fds) {
    int err = 0;

    uint64_t to_child[2] = {0};
    uint64_t to_parent[2] = {0};
    err = ckb_pipe(to_child);
    if (err != 0) {
        return err;
    }
    err = ckb_pipe(to_parent);
    if (err != 0) {
        return err;
    }
    inherited_fds[0] = to_child[0];
    inherited_fds[1] = to_parent[1];
    inherited_fds[2] = 0;
    fds[0] = to_parent[0];
    fds[1] = to_child[1];
    return 0;
}

int main() {
    int err = 0;
    const char *argv[] = {"hello", "world"};
    uint64_t pid = 0;
    uint64_t fds[2] = {0};
    uint64_t inherited_fds[3] = {0};
    err = create_std_fds(fds, inherited_fds);
    if (err != 0) {
        return err;
    }
    spawn_args_t spgs = {
        .argc = 2,
        .argv = argv,
        .process_id = &pid,
        .inherited_fds = inherited_fds,
    };
    err = ckb_spawn(1, CKB_SOURCE_CELL_DEP, 0, 0, &spgs);
    if (err != 0) {
        return err;
    }
    uint8_t buffer[1024] = {0};
    size_t length = 1024;
    err = ckb_read(fds[0], buffer, &length);
    if (err != 0) {
        return err;
    }
    err = memcmp("helloworld", buffer, length);
    if (err != 0) {
        return err;
    }
    return 0;
}
