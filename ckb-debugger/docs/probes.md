Tracing the running of CKB Contracts with User-Level Statically Defined Tracing (USDT) probes.

USDT probes (tracing points) are a powerful tool for debugging and tracing the running of a program. USDT stands for User-Level Statically Defined Tracing.
USDT probes allow developers to insert tracepoints into their code, which can be used to monitor program execution and diagnose problems.
We have defined a few USDT tracing points in ckb-debugger to trace the execution of CKB contracts.
For example, whenever CKB contracts jump to a new function, we can print out the parameters of this function call.

# USDT Probes

## bcc and bpftrace
To leverage USDT probes to trace the running of a program, we can use utiltilies like [bcc](https://github.com/iovisor/bcc/) and [bpftrace](https://github.com/iovisor/bpftrace/).
Here is a crash course for the basics of each tool. Bcc is a set of tools for kernel tracing and analysis,
which includes a library of tracepoints and a Python interface for creating custom tracepoints.
Bpftrace is a high-level tracing language for Linux eBPF (Extended Berkeley Packet Filter)
that allows you to write scripts to trace events and gather data from the kernel.

Both bcc and bpftrace have their advantages and disadvantages when it comes to using USDT probes.
Bcc is a more mature tool with a larger library of tracepoints. Along with the python module, bcc is much more versatile.
Bpftrace, on the other hand, is a more flexible tool that allows you to write scripts in a high-level language,
which can be more expressive and easier to read.

We can use bpftrace to get our hands dirty and then use bcc with python when the functionality of bpftrace is not enough.
For example, to trace the `do_sys_open` function in the Linux kernel, we can use the following script:

```
uprobe:/usr/src/linux/fs/open.c:do_sys_open
{
    printf("do_sys_open called\n");
}
```

This script will print out a message every time the `do_sys_open` function is called in the kernel.

To use bcc with python, we need to familiarize ourself with bcc's interface (an example python script is listed below).

## Probes in ckb-debugger

We can build `ckb-debugger` with USDT by running `cargo build -p ckb-debugger --features probes --release`.
Afterwards, we can list all the defined tracing points with `sudo bpftrace -l 'usdt:./target/release/ckb-debugger:*'`

```
usdt:./target/release/ckb-debugger:ckb_vm:execute_inst
usdt:./target/release/ckb-debugger:ckb_vm:execute_inst_end
```

Currently there are two static tracing points defined in ckb-debugger.
One (`execute_inst`) is used to track the execution of all instructions,
and another trace point (`execute_inst_end`) is reached when the execution of one instruction has finished).
Both of they have exposed the following variables
- `pc`, value of current program counter
- `cycles`, cycles consumed so far
- `inst`, current instruction (note this instruction is in an internal representation)
- `regs`, the start address of ckb-vm's registers values in the host
- `memory`, the start address of ckb-vm's memory in the host

`execute_inst_end` has an additionally parameter to indicate the return value of the instruction execution.
It is 0 if execution of the instruction has finished normally.

# Usage of ckb-debugger Probes
Below we give a brief introduction on how to trace programs on ckb-vm with bpftrace and bcc,
more examples are available in [xxuejie/ckb-vm-bpf-toolkit](https://github.com/xxuejie/ckb-vm-bpf-toolkit).

## Tracing Istructions
Say we want to trace a syscall. Since the instruction of syscall `ecall` is encoded into `0x2000023` (the details of this instruction encoding can be found at [ckb-vm/instructions.rs](https://github.com/nervosnetwork/ckb-vm/blob/b7d75770875f27ea1ca15ab11cd14a01c7f19f38/definitions/src/instructions.rs)), we can run the following command to trace any syscall (assuming `./target/release/ckb-debugger` is the path of ckb-debugger).

```
sudo bpftrace -e 'usdt:./target/release/ckb-debugger:ckb_vm:execute_inst /arg2 == 0x2000023/ { printf("%p, %d, %p, %p, %p\n", arg0, arg1, arg2, arg3, arg4); }'
```

An example output of the above command is

```
Attaching 1 probe...
0x11ffa, 78788, 0x2000023, 0x7ffdbea9bed8, 0x7fb6ff935010
0x11ffa, 1644225, 0x2000023, 0x7ffdbea9bed8, 0x7fb6ff935010
0x11ffa, 1645644, 0x2000023, 0x7ffdbea9bed8, 0x7fb6ff935010
0x15b68, 1968395, 0x2000023, 0x7ffdbea9bed8, 0x7fb6ff935010
```

The second line of the output `0x11ffa, 78788, 0x2000023, 0x7ffdbea9bed8, 0x7fb6ff935010` represents
the program counter is 0x11ffa, current consumed cycles are 78788, the running instruction is 0x2000023, the start address of registers content is 0x7ffdbea9bed8 and the start address of memory content is 0x7fb6ff935010.

In the same vein, we can use the trace point `execute_inst_end` to trace the result of any syscall.

In order to do more complex work like dumping the syscall number and dumping memory content, we need to use more sophisticated techniques like using bcc python module. Below is an example of tracing function calls/returns with bcc.

## Tracing Functions Calls/Returns

### Obtaining the Memory Address Range of Some Function
More work is needed when we want to trace functions return values and parameters of a ckb-vm programs.
Ckb-vm only know the instructions which is about to execute, we need to parse the binary file to find out
the context of the instructions (which function does this instruction belongs). This
may be done by something like [`addr2line`](https://linux.die.net/man/1/addr2line).
To do this programatically, we can use The file [elfutils.py](../examples/elfutils.py) to obtain memory address range for a function.

### How Tracing Function Calls and Returns Works
In order to understand the workflow of tracing function calls and returns. 
We need to understand how to do control flow transfer in RISC-V.
In the RISC-V ISA there are two unconditional control transfer instructions: `jalr`,
which jumps to an absolute address as specified by an immediate offset from a register;
and `jal`, which jumps to a pc-relative offset as specified by an immediate.
Whenever there is a jal/jalr instruction, we save the link address (an address that this jump started from) to a hash map.
If the destination address of next jal/jalr instruction was found in the hash map,
then this instruction is likely a function return. Otherwise, this may be a function call.

### Real World Function Calls/Returns Example

It can trace the parameters and return values of the function `test_func` of the below program.
```
int* test_func(int parameter1, int parameter2) {
    int *mem = (int *)malloc(sizeof(parameter1) + sizeof(parameter2));
    mem[0] = parameter1;
    mem[1] = parameter2;
    return mem;
}

int main(int argc, char **argv) {
    free((void *) test_func(42, 43));
    return 0;
}
```

To trace the function calling and returning, 
We can run `sudo ./example/trace.py`. Below is a sample output of running this script.

```
Executed jumping-related instructions 57874 times!
Func test_func has been jumped to/from 2 times!
Func test_func has been called 1 times!
Func test_func has returned 1 times!
Dumping value counts of func test_func parameter 1
parameter 1 value: 000000000000002a, count: 1
Dumping value counts of func test_func parameter 2
parameter 2 value: 000000000000002b, count: 1
Dumping return value counts for func test_func
return value: 000000000005d020, count: 1
Dumping meomry content located at the return value of func test_func
memory addr: 000000000005d020, content: 0000002b0000002a
```

As we can see, both the parameter passed to the function `test_func` and the return value of this function can be traced.
Even better, we can dump the memory content located at the return address of `test_func`.
BCC is so flexible that you may easily tweak this script to meet your needs. Feel free to modify this script.

### Walkthrough of Tracing Function Calls/Returns
The script [trace.py](../examples/trace.py) implements such logic to trace function calls/returns.
Refer to [bcc/reference_guide.md](https://github.com/iovisor/bcc/blob/master/docs/reference_guide.md) for the BCC python API
and [RISC-V Specifications](https://riscv.org/technical/specifications/) for the detailed meaning of
each instruction and their operands.

We first decode the instruction obtained from trace point `execute_inst`.
Here we early return on finding that the instruction currently running is not a jump instruction.
If the instruction is indeed a jump related instruction then we save the link
(the return address that we will jump to when this jump instruction finishes, normally 
`current_pc` + `instruction_length`) and next program counter to jump to.
Note here in addition to jal/jalr we also have two pseudo-instructions `OP_FAR_JUMP_ABS` and `OP_FAR_JUMP_REL`
to jump to another function. They are [https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0033-ckb-vm-version-1/0033-ckb-vm-version-1.md#42-mon](macro-operation fusion) to improve the performance.

```c
    InstructionOpcode opcode = EXTRACT_OPCODE(instruction);
    uint8_t instruction_length = INSTRUCTION_LENGTH(instruction);
    // The return address that we will jump to when this jump instruction finishes,
    // normally current_pc + instruction_length.
    uint64_t link;
    // The address that this jump instruction will jump to.
    uint64_t next_pc;
    SImmediate imm;
    RegisterIndex ind;

    // Decode the instuction to get function calls/returns information.
    switch (opcode)
    {
        case OP_JAL:
            link = pc + instruction_length;
            imm = UTYPE_IMMEDIATE_S(instruction);
            next_pc = pc + imm;
            break;
        case OP_JALR_VERSION0:
        case OP_JALR_VERSION1:
            link = pc + instruction_length;
            imm = ITYPE_IMMEDIATE_S(instruction);
            ind = ITYPE_RS1(instruction);
            uint64_t reg_value = 0;
            bpf_probe_read_user(&reg_value, sizeof(uint64_t), (void *)(regs_addr + sizeof(uint64_t) * ind));
            next_pc = (reg_value + imm) & ~1;
            break;
        case OP_FAR_JUMP_ABS:
            link = pc + instruction_length;
            imm = UTYPE_IMMEDIATE_S(instruction);
            next_pc = imm & ~1;
            break;
        case OP_FAR_JUMP_REL:
            link = pc + instruction_length;
            imm = UTYPE_IMMEDIATE_S(instruction);
            next_pc = (pc + imm) & ~1;
            break;
        default:
            return 0;
    }
```

We then determine if this jump is a function call or return by checking if the jumping-to address is the
start of the function and looking up the hash table of all link addresses.
If the next pc is saved in the hash table, that is to say it is been previously linked and is now returning to it,
this is a function return.

```c
    int is_calling = 0;
    int is_returning = 0;
    if (next_pc == @@LOW_PC@@) {
        // Initialize reference of the link, increment refcount if neccesary. 
        jump_from_addresses.increment(link);
        is_calling = 1;
    }

    if (link > @@LOW_PC@@ && link <= @@HIGH_PC@@) {
        uint64_t *refcount = jump_from_addresses.lookup(&next_pc);
        (*refcount)--;
        if (*refcount == 0) {
            jump_from_addresses.delete(&next_pc);
        }
        is_returning = 1;
    }
```

We load the function parameters and return values by `bpf_probe_read_user` and then save them
into bpf tables which can then be read by userspace programs (in our case the main python script).
Memory contents at the address `ret` can read by first loading the start addres with `bpf_usdt_readarg(5, ctx, &mem_addr)`
and then reading the content with `bpf_probe_read_user(&content, sizeof(uint64_t), (void *)(mem_addr + ret));`
Finally, memory content is saved to the bpf table `memory_contents`.

```c
    uint64_t mem_addr = 0;
    bpf_usdt_readarg(5, ctx, &mem_addr);

    uint64_t ret = 0;
    bpf_probe_read_user(&ret, sizeof(uint64_t), (void *)(regs_addr + 8 * A0));

    uint64_t content = 0;
    bpf_probe_read_user(&content, sizeof(uint64_t), (void *)(mem_addr + ret));
    memory_contents.update(&ret, &content);
```

Finally we can iterate over the bpf tables and print out their values.

```python
for k, v in sorted(b.get_table("memory_contents").items(), key=lambda kv: kv[0].value):
    print(f"memory addr: {k.value:016x}, content: {v.value:016x}")
```
