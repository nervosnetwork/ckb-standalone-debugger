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
usdt:./target/release/ckb-debugger:ckb_vm:jump
usdt:./target/release/ckb-debugger:ckb_vm:syscall
usdt:./target/release/ckb-debugger:ckb_vm:syscall_ret
```

Currently there are three major categories of static tracing points defined in ckb-debugger.
One is used to track the execution of all instructions (including `execute_inst` and `execute_inst_end`),
another is used track the running and returning of syscalls (including `syscall` and `syscall_ret`). 
The final one (`jump`) is used to track riscv jump instructions (including both `JAL` and `JARL`).
We now explain how to use each of these probes.

# Usage of ckb-debugger Probes
Below we give a brief introduction on how to trace programs on ckb-vm with bpftrace and bcc,
more examples are available in [xxuejie/ckb-vm-bpf-toolkit](https://github.com/xxuejie/ckb-vm-bpf-toolkit).

## Tracing Istructions
TODO

## Tracing Syscalls
[Syscalls](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0009-vm-syscalls/0009-vm-syscalls.md) are easy to trace with USDT probes.
We assumes some basic knowledge of how syscalls in ckb-vm works. If there are some unfamiliar concepts,
you can refer to [rfcs/0009-vm-syscalls.md](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0009-vm-syscalls/0009-vm-syscalls.md) for
the conventions used in the ckb-vm syscalls.

To print the arguments to the syscall with number 2061 (`ckb_load_tx_hash`), we can just run bpftrace
```
sudo bpftrace -e 'usdt:./target/release/ckb-debugger:ckb_vm:syscall /arg0 == 2061/ { printf("%d, %d, %d, %d, %d, %d\n", arg0, arg1, arg2, arg3, arg4, arg5); }'
```

Here arg0 is the syscall number, and arg1, arg2, arg3, arg4, args5 are the values of registers A0, A1, A2, A3, A4
(which are arguments passed to the syscall, note due to the limitation to the number of arguments of a USDT probe,
the content of A5 is not exposed). A sample output is

```
Attaching 1 probe...
2061, 0, 4188960, 0, 0
2061, 399744, 4188960, 0, 0
```

Another thing we can do is to trace the return value of a syscall.
To trace the return values to the syscall `ckb_load_tx_hash`, we can run
```
sudo bpftrace -e 'usdt:./target/release/ckb-debugger:ckb_vm:syscall_ret /arg0 == 2061/ { printf("%d, %d, %d, %d, %d\n", arg0, arg1, arg2); }'
```
Here arg0 is the syscall number, and arg1, arg2 are the values of registers A0, A1 (which are return values of the syscall).

## Tracing Functions Calls/Returns
More work is needed when we want to trace functions return values and parameters of a ckb-vm programs.
Ckb-vm only know the instructions which is about to execute, we need to parse the binary file to find out
the context of the instructions (which function does this instruction belongs). This
may be done by something like [`addr2line`](https://linux.die.net/man/1/addr2line).
To do this programatically, we can use The file [elfutils.py](../examples/elfutils.py) to obtain memory address range for a function.

In order to understand the workflow of tracing function calls and returns. 
We need to understand how to do control flow transfer in RISC-V.
In the RISC-V ISA there are two unconditional control transfer instructions: jalr,
which jumps to an absolute address as specified by an immediate offset from a register;
and jal, which jumps to a pc-relative offset as specified by an immediate.
Whenever there is a jal/jalr instruction, we save the link address (an address that this jump started from) to a hash map.
If the destination address of next jal/jalr instruction was found in the hash map,
then this instruction is likely a function return. Otherwise, this may be a function call.

The script [trace.py](../examples/trace.py) implements such logic.
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
