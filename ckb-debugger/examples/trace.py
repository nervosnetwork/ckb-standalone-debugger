#!/usr/bin/env python3

# Trace the calling and returning of a specific function.
#
# Usage:
# Run this script with root permission. You may want to modify the variable below.

import os
import ctypes

from subprocess import Popen, PIPE

from elfutils import get_function_address_range
from bcc import BPF, USDT
from elftools.elf.elffile import ELFFile

# Change the path of ckb-debugger, the path of the program to run and the function name to probe here.
BIN_TO_TRACE = "/home/e/Workspace/ckb-lua/build/lua-loader.debug"
CKB_DEBUGGER_ARGUMENTS = [
    "/home/e/.cargo/bin/ckb-debugger",
    "--mode",
    "probe",
    "--prompt", # prompt suspend the execution of ckb-debugger until a new line is passed to stdin
    "--bin",
    BIN_TO_TRACE,
    # You may add extra arguments here
]
FUNC_TO_PROBE = "^test_func$"

# Obtain memory address range for function
elf = ELFFile(open(BIN_TO_TRACE, "rb"))
func_range = get_function_address_range(elf, FUNC_TO_PROBE)
if func_range is None:
    print("Range for function %s not found" % (FUNC_TO_PROBE))
    exit(1)
func_name, func_low_pc, func_high_pc = func_range[0], func_range[1], func_range[2]

print("Profiling func %s" % (func_name))

# Below is th BPF program to run
bpf_text = """
#include "riscv.h"

BPF_HASH(num_of_effective_jumps, uint64_t);
BPF_HASH(num_of_calling, uint64_t);
BPF_HASH(num_of_returning, uint64_t);
BPF_HASH(return_values, uint64_t);
BPF_HASH(memory_contents, uint64_t);
BPF_HASH(memory_addrs, uint64_t);
// hash map that maps the link addresses to the reference counts
BPF_HASH(jump_from_addresses, uint64_t);
BPF_HASH(parameter1_counts, uint64_t);
BPF_HASH(parameter2_counts, uint64_t);

int do_jump(struct pt_regs *ctx) {
    // link is always current_pc + instruction_length()
    // Initialize link, so that bpf verifier does not report error like R8 !read_ok
    uint64_t link = 0;
    bpf_usdt_readarg(1, ctx, &link);

    // next_pc is the pc address that this jump instruction intends to jump to.
    // Initialize link, so that bpf verifier does not report error like R8 !read_ok
    uint64_t next_pc = 0;
    bpf_usdt_readarg(2, ctx, &next_pc);

    // x calls a, link = current address in x + instruction_length(), next_pc = start address of a
    // y returns to a, link = current address in y + instruction_length(), next_pc = some address of a

    int is_calling = 0;
    int is_returning = 0;
    if (next_pc == @@LOW_PC@@) {
        // Initialize reference of the link, increment refcount if neccesary. 
        jump_from_addresses.increment(link);
        is_calling = 1;
    }

    // TODO: here is an edge case. Say the ret instruction with memory address ret_addr
    // is in the end of the function func_a. When func_a returns from ret_addr, then
    // link = mem_ret + instruction_length() which equals high pc of func_a, i.e. @@HIGH_PC@@.
    // So we must also check next_pc == @@HIGH_PC@@, but @@HIGH_PC@@ may be the start of another
    // function.
    if (link > @@LOW_PC@@ && link <= @@HIGH_PC@@) {
        uint64_t *refcount = jump_from_addresses.lookup(&next_pc);
        if (refcount == NULL) {
            // Should be unreachable
            return 1;
        }
        (*refcount)--;
        if (*refcount == 0) {
            jump_from_addresses.delete(&next_pc);
        }
        is_returning = 1;
    }

    if (is_returning == 0 && is_calling == 0) {
        return 0;
    }

    num_of_effective_jumps.increment(1);

    uint64_t regs_addr = 0;
    bpf_usdt_readarg(3, ctx, &regs_addr);

    uint64_t mem_addr = 0;
    bpf_usdt_readarg(4, ctx, &mem_addr);

    if (is_calling == 1) {
        num_of_calling.increment(1);
        uint64_t a0 = 0;
        bpf_probe_read_user(&a0, sizeof(uint64_t), (void *)(regs_addr + 8 * A0));
        parameter1_counts.increment(a0);

        uint64_t a1 = 0;
        bpf_probe_read_user(&a1, sizeof(uint64_t), (void *)(regs_addr + 8 * A1));
        parameter2_counts.increment(a1);
    }
    if (is_returning == 1) {
        num_of_returning.increment(1);
        uint64_t ret = 0;
        bpf_probe_read_user(&ret, sizeof(uint64_t), (void *)(regs_addr + 8 * A0));

        uint64_t zero_value = 0;
        return_values.lookup_or_try_init(&ret, &zero_value);
        return_values.increment(ret);

        uint64_t content = 0;
        bpf_probe_read_user(&content, sizeof(uint64_t), (void *)(mem_addr + ret));
        memory_contents.update(&ret, &content);

        memory_addrs.increment(mem_addr);
    }

    return 0;
}
"""

bpf_text = bpf_text.replace("@@LOW_PC@@", str(func_low_pc)).replace(
    "@@HIGH_PC@@", str(func_high_pc)
)

print("bpf program source code:")
print(bpf_text)

# Run the ckb-debugger and attach a BPF program to the process. 
p = Popen(CKB_DEBUGGER_ARGUMENTS, stdin=PIPE)
u = USDT(pid=int(p.pid))
u.enable_probe(probe="ckb_vm:jump", fn_name="do_jump")
include_path = os.path.dirname(os.path.abspath(__file__))
b = BPF(
    text=bpf_text,
    usdt_contexts=[u],
    cflags=["-Wno-macro-redefined", "-I", include_path],
)

# Prime ckb-debugger to execute the program
p.communicate(input="\n".encode())

# Dump the inforamtion saved by the BPF program
called = b["num_of_effective_jumps"][ctypes.c_ulong(1)].value
print("Func %s has been jumped to/from %s times!" % (func_name, called))
called = b["num_of_calling"][ctypes.c_ulong(1)].value
print("Func %s has been called %s times!" % (func_name, called))
called = b["num_of_returning"][ctypes.c_ulong(1)].value
print("Func %s has returned %s times!" % (func_name, called))

print("Dumping value counts of func %s parameter 1" % (func_name))
for k, v in sorted(b.get_table("parameter1_counts").items(), key=lambda kv: kv[0].value):
    print(f"parameter 1 value: {k.value:016x}, count: {v.value:}")
print("Dumping value counts of func %s parameter 2" % (func_name))
for k, v in sorted(b.get_table("parameter2_counts").items(), key=lambda kv: kv[0].value):
    print(f"parameter 2 value: {k.value:016x}, count: {v.value:}")

print("Dumping return value counts for func %s" % (func_name))
for k, v in sorted(b.get_table("return_values").items(), key=lambda kv: kv[0].value):
    print(f"return value: {k.value:016x}, count: {v.value:}")
print("Dumping meomry content located at the return value of func %s" % (func_name))
for k, v in sorted(b.get_table("memory_contents").items(), key=lambda kv: kv[0].value):
    print(f"memory addr: {k.value:016x}, content: {v.value:016x}")
