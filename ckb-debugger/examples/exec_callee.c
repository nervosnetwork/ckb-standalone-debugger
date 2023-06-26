#include "ckb_syscalls.h"

int main(int argc, char* argv[]) {
  ckb_debug("exec_callee");
  if (argc != 3) {
    return 1;
  }
  if (argv[0][0] != 'a') {
    return 2;
  }
  if (argv[1][0] != 'b') {
    return 3;
  }
  if (argv[2][0] != 'c') {
    return 4;
  }
  return 0;
}
