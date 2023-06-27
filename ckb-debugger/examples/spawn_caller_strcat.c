#include <stdint.h>
#include <string.h>

#include "ckb_syscalls.h"

int main() {
  const char*  argv[] = {"hello", "world"};
  int8_t       spgs_exit_code = 255;
  uint8_t      spgs_content[80] = {};
  uint64_t     spgs_content_length = 80;
  spawn_args_t spgs = {
    .memory_limit = 8,
    .exit_code = &spgs_exit_code,
    .content = &spgs_content[0],
    .content_length = &spgs_content_length,
  };
  int success =
      ckb_spawn(1, 3, 0, 2, argv, &spgs);
  if (success != 0) {
    return 1;
  }
  if (spgs_exit_code != 0) {
    return 1;
  }
  if (strlen((char *)spgs_content) != 10) {
    return 1;
  }
  if (strcmp((char *)spgs_content, "helloworld") != 0) {
    return 1;
  }
  return 0;
}
