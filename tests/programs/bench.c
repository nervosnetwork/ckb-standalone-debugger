#include <stdlib.h>

#include "ckb_syscalls.h"

int main(int argc, char* argv[]) {
  char buffer[1024 * 1024];
  if (argc != 5) {
    return -1;
  }

  uint64_t adds = atoi(argv[1]);
  uint64_t muls = atoi(argv[2]);
  uint64_t loads = atoi(argv[3]);
  uint64_t load_bytes = atoi(argv[4]);

  uint64_t result = 0;
  for (uint64_t i = 1; i <= adds; i++) {
    result += adds;
  }
  for (uint64_t i = 1; i <= muls; i++) {
    result *= muls;
  }
  for (uint64_t i = 1; i <= loads; i++) {
    uint64_t len = load_bytes;
    uint64_t ret = ckb_load_cell_data(buffer, &len, 0, 0, CKB_SOURCE_CELL_DEP);
    if (ret != 0) {
      return ret;
    }
    result += buffer[0];
  }

  if (result == 0) {
    return -1;
  }

  return 0;
}
