#include "ckb_syscalls.h"
#include "protocol.h"

int main() {
  char script[1024];
  char buffer[1024 * 1024];

  uint64_t len = 1024;
  uint64_t ret = ckb_load_script(script, &len, 0);
  if (ret != 0) {
    return ret;
  }
  mol_seg_t script_seg;
  script_seg.ptr = (uint8_t*)script;
  script_seg.size = len;
  mol_seg_t args_seg = MolReader_Script_get_args(&script_seg);
  mol_seg_t args_bytes_seg = MolReader_Bytes_raw_bytes(&args_seg);
  if (args_bytes_seg.size != 32) {
    return -101;
  }
  uint64_t adds = *((const uint64_t *) (&args_bytes_seg.ptr[0]));
  uint64_t muls = *((const uint64_t *) (&args_bytes_seg.ptr[8]));
  uint64_t loads = *((const uint64_t *) (&args_bytes_seg.ptr[16]));
  uint64_t load_bytes = *((const uint64_t *) (&args_bytes_seg.ptr[24]));

  uint64_t result = 0;
  for (uint64_t i = 1; i <= adds; i++) {
    result += adds;
  }
  for (uint64_t i = 1; i <= muls; i++) {
    result *= muls;
  }
  for (uint64_t i = 1; i <= loads; i++) {
    len = load_bytes;
    ret = ckb_load_cell_data(buffer, &len, 0, 0, CKB_SOURCE_CELL_DEP);
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
