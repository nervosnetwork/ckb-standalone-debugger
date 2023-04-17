Tracing the running of CKB Contracts with User-Level Statically Defined Tracing.

USDT probes are a powerful tool for debugging and tracing the running of a program. USDT stands for User-Level Statically Defined Tracing. USDT probes allow developers to insert tracepoints into their code, which can be used to monitor program execution and diagnose problems. We have defined a few USDT tracing points in ckb-debugger to trace the execution of CKB contracts. For example, whenever CKB contracts jump to a new function, we can print out the parameters of this function call.

# usdt

## bcc and bpftrace
To leverage USDT probes to trace the running of a program, we can use utiltilies like bcc and bpftrace.
Here is a crash course for the basics of each tool. Bcc is a set of tools for kernel tracing and analysis, which includes a library of tracepoints and a Python interface for creating custom tracepoints. Bpftrace is a high-level tracing language for Linux eBPF (Extended Berkeley Packet Filter) that allows you to write scripts to trace events and gather data from the kernel.

Both bcc and bpftrace have their advantages and disadvantages when it comes to using USDT probes. Bcc is a more mature tool with a larger library of tracepoints. Along with the python module, bcc is much more versatile. Bpftrace, on the other hand, is a more flexible tool that allows you to write scripts in a high-level language, which can be more expressive and easier to read.

We can use bpftrace to get our hands dirty and then use bcc with python when the functionality of bpftrace is not enough. For example, to trace the `do_sys_open` function in the Linux kernel, we can use the following script:

```
uprobe:/usr/src/linux/fs/open.c:do_sys_open
{
    printf("do_sys_open called\n");
}
```

This script will print out a message every time the `do_sys_open` function is called in the kernel.

To use bcc with python, we need to familiarize ourself with bcc's interface (an example usage is listed below).


## tracing points

We can build `ckb-debugger` with USDT by running `cargo build -p ckb-debugger --features probes --release`.
Afterwards, we can list all the defined tracing points with `sudo bpftrace -l 'usdt:./target/release/ckb-debugger:*'`

```
usdt:./target/release/ckb-debugger:ckb_vm:execute_inst
usdt:./target/release/ckb-debugger:ckb_vm:execute_inst_end
usdt:./target/release/ckb-debugger:ckb_vm:jump
usdt:./target/release/ckb-debugger:ckb_vm:syscall
usdt:./target/release/ckb-debugger:ckb_vm:syscall_ret
```

Currently there are three major categories of static tracing points in ckb-debugger. One is used to track the execution of all instructions (`execute_inst`, `execute_inst_end`), another is used track the running and returning of syscalls (`syscall`, `syscall_ret`). The final one (`jump`) is used to track riscv jump instructions (including both `JAL` and `JARL`). We now explain how to use each of these probes.

# usage

## instructions
TODO

## syscalls
Syscalls are easier to trace, to print the arguments to the syscall, we can just run bpftrace
```
sudo bpftrace -e 'usdt:./target/release/ckb-debugger:ckb_vm:syscall /arg0 == 2061/ { printf("%d, %d, %d, %d, %d\n", arg0, arg1, arg2, arg3, arg4); }'
```

Here arg0 is the syscall number, and arg1, arg2, arg3, arg4 are arguments to the syscall. A sample output is
```
Attaching 1 probe...
2061, 0, 4188960, 0, 0
2061, 399744, 4188960, 0, 0
```

## functions
More work is needed when we want to trace functions return values and parameters of a ckb-vm programs.
Ckb-vm only know the instructions which is about to execute, we need to parse the binary file to find out
the context of the instructions (which function does this instruction belongs). This
may be done by something like [`addr2line`](https://linux.die.net/man/1/addr2line).
To do this programatically, we can use The file [elfutils.py](../examples/elfutils.py) to obtain memory address range for a function.

Say we want to trace the parameters and return values of the function `test_func`.
```
long* test_func(long *mem, long parameter1, long parameter2) {
    long *mem = (long *)malloc(sizeof(parameter1) + sizeof(parameter2));
    mem[0] = parameter1;
    mem[1] = parameter2;
    return mem;
}

int main(int argc, char **argv) {
    long mem[2];
    free(test_func());
    return 0;
}
```

In order to understand the workflow of tracing function calls and returns. We need to understand how to do control flow transfer in RISC-V.
In the RISC-V ISA there are two unconditional control transfer instructions: jalr, which jumps to an absolute address as specified by an immediate offset from a register; and jal, which jumps to a pc-relative offset as specified by an immediate. Whenever there is a jal/jalr instruction, we save the link address (an address that this jump started from) to a hash map.
If the destination address of next jal/jalr instruction was found in the hash map, then this instruction is likely a function return. Otherwise, this may be a function call.

The script [trace.py](../examples/trace.py) implements such logic. To trace the function calling and returning, 
We can run `sudo env RUST_LOG=debug ./example/trace.py --bin ./example-bin --bpf-func "^test_func$" --bpf-debugger ../ckb-standalone-debugger/target/release/ckb-debugger`. Below is a sample output of running this script.

```
Func test_func has been jumped to/from 2 times!
Func test_func has been called 1 times!
Func test_func has returned 1 times!
Dumping return value counts for func test_func
return value: 000000000005d020, count: 1
Dumping meomry addresses for func test_func
memory addr: 00007f11c6b4b010, content: 0000000000000001
Dumping meomry content for func test_func
memory addr: 000000000005d020, content: 31302f2e2d2c2b2a
```
