#include "ckb_syscalls.h"
#include "protocol.h"

int main()
{
  ckb_debug("I'm in main now!");

  char script[1024];
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

  if (args_bytes_seg.size == 3) {
    return -2;
  } else if (args_bytes_seg.size == 5 ) {
   return -3;
  } else {
   return 0;
  }
}
