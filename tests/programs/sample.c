#include "ckb_syscalls.h"
#include "protocol_reader.h"

int main()
{
  ckb_debug("I'm in main now!");

  char script[1024];
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
  if (bytes_res.code != 0) {
    return -101;
  }

  if (bytes_res.pos.size == 3) {
    return -2;
  } else if (bytes_res.pos.size == 5 ) {
   return -3;
  } else {
   return 0;
  }
}
