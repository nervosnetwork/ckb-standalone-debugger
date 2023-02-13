_abc:
	riscv64-unknown-elf-gcc -g -o abc abc.c

_fib:
	riscv64-unknown-elf-gcc -g -o fib fib.c

_sprintf:
	riscv64-unknown-elf-gcc -g -o sprintf sprintf.c

_outofmemory:
	riscv64-unknown-elf-gcc -g -o outofmemory outofmemory.c

_sbrk:
	riscv64-unknown-elf-gcc -g -o sbrk sbrk.c

abc:
	docker run --rm -v `pwd`:/code nervos/ckb-riscv-gnu-toolchain:bionic-20210804 bash -c "cd /code && make _abc"

fib:
	docker run --rm -v `pwd`:/code nervos/ckb-riscv-gnu-toolchain:bionic-20210804 bash -c "cd /code && make _fib"

sprintf:
	docker run --rm -v `pwd`:/code nervos/ckb-riscv-gnu-toolchain:bionic-20210804 bash -c "cd /code && make _sprintf"

outofmemory:
	docker run --rm -v `pwd`:/code nervos/ckb-riscv-gnu-toolchain:bionic-20210804 bash -c "cd /code && make _outofmemory"

sbrk:
	docker run --rm -v `pwd`:/code nervos/ckb-riscv-gnu-toolchain:bionic-20210804 bash -c "cd /code && make _sbrk"
