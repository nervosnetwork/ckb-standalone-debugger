#include "ckb_syscalls.h"
#include "protocol_reader.h"

int main() {
  char script[1024];
  char buffer[1024 * 1024];

  volatile uint64_t len = 1024;
  uint64_t ret = ckb_load_script(script, &len, 0);
  if (ret != 0) {
    return ret;
  }
  mol_pos_t script_pos;
  script_pos.ptr = (const uint8_t*)script;
  script_pos.size = len;
  mol_read_res_t args_res = mol_cut(&script_pos, MOL_Script_args());
  if (args_res.code != 0) {
    return -100;
  }
  mol_read_res_t bytes_res = mol_cut_bytes(&args_res.pos);
  if (bytes_res.code != 0 || bytes_res.pos.size != 32) {
    return -101;
  }
  uint64_t adds = *((const uint64_t *) (&bytes_res.pos.ptr[0]));
  uint64_t muls = *((const uint64_t *) (&bytes_res.pos.ptr[8]));
  uint64_t loads = *((const uint64_t *) (&bytes_res.pos.ptr[16]));
  uint64_t load_bytes = *((const uint64_t *) (&bytes_res.pos.ptr[24]));

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
