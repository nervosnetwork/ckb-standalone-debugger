CC := clang-18 --target=riscv64 -march=rv64imac_zba_zbb_zbc_zbs -g -nostdinc -nostdlib -Ickb-c-stdlib -Ickb-c-stdlib/libc

all: ckb-c-stdlib \
	 always_failure \
	 exec_callee \
	 exec_caller \
	 fib \
	 out_of_memory \
	 print_log \
	 spawn_callee_strcat \
	 spawn_caller_strcat \
	 timenow

ckb-c-stdlib:
	git clone https://github.com/nervosnetwork/ckb-c-stdlib
	cd ckb-c-stdlib
	git checkout d7dc1e3
	cd ..

always_failure:
	$(CC) -o $@ $@.c

exec_callee:
	$(CC) -o $@ $@.c

exec_caller:
	$(CC) -o $@ $@.c

fib:
	$(CC) -o $@ $@.c

out_of_memory:
	$(CC) -o $@ $@.c

print_log:
	$(CC) -o $@ $@.c

spawn_callee_strcat:
	$(CC) -o $@ $@.c

spawn_caller_strcat:
	$(CC) -o $@ $@.c

timenow:
	$(CC) -o $@ $@.c

test:
	bash MakefileTest.sh
