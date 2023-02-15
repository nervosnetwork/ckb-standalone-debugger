TARGET := riscv64-unknown-elf
CC := $(TARGET)-gcc
LD := $(TARGET)-gcc
OBJCOPY := $(TARGET)-objcopy
CFLAGS := -O0 -fno-builtin-printf -nostdinc -nostdlib -nostartfiles -I ./ckb-c-stdlib/libc -I ./ckb-c-stdlib -g
LDFLAGS := -Wl,-static
BUILDER_DOCKER := nervos/ckb-riscv-gnu-toolchain:jammy-20230214

CKB-DEBUGGER := ../../target/debug/ckb-debugger
all: build ckb-c-stdlib build/fib build/outofmemory build/read_file build/file_operations

all-via-docker:
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make"

build:
	[ ! -f build ] && mkdir -p build

ckb-c-stdlib:
	[ ! -f ckb-c-stdlib ] && git clone https://github.com/nervosnetwork/ckb-c-stdlib

build/fib: fib.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

build/outofmemory: outofmemory.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

build/read_file: read_file.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

build/file_operations: file_operations.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

fmt:
	clang-format -style="{BasedOnStyle: google, IndentWidth: 4, SortIncludes: false}" -i *.c

clean:
	rm -rf build
	rm -rf ckb-c-stdlib

run-read-file: build/read_file
	RUST_LOG=debug $(CKB-DEBUGGER) --bin build/read_file --read-file read_file.c

run-file-operations: build/file_operations
	RUST_LOG=debug $(CKB-DEBUGGER) --bin build/file_operations

run:
# mode=full
	$(CKB-DEBUGGER) --tx-file=../../ckb-debugger-api/tests/programs/sample.json --script-group-type=type \
		--script-hash=0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c --script-version=0
	$(CKB-DEBUGGER) --tx-file=../../ckb-debugger-api/tests/programs/sample_data1.json --script-group-type=type \
		--script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
# mode=full pprof=build/flamegraph.txt
	$(CKB-DEBUGGER) --pprof build/flamegraph.txt --tx-file=../../ckb-debugger-api/tests/programs/sample.json \
		--script-group-type=type --script-hash=0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c --script-version=0
	$(CKB-DEBUGGER) --pprof build/flamegraph.txt --tx-file=../../ckb-debugger-api/tests/programs/sample_data1.json \
		--script-group-type=type --script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
# mode=fast
	$(CKB-DEBUGGER) --mode fast --tx-file=../../ckb-debugger-api/tests/programs/sample.json --script-group-type=type \
		--script-hash=0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c --script-version=0
	$(CKB-DEBUGGER) --mode fast --tx-file=../../ckb-debugger-api/tests/programs/sample_data1.json --script-group-type=type \
		--script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
# mode=full pprof=build/flamegraph.txt
	RUST_LOG=trace $(CKB-DEBUGGER) --bin=build/fib --pprof=build/flamegraph.txt \
		--tx-file=../../ckb-debugger-api/tests/programs/sample_data1.json --script-group-type=type \
		--script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
	cat build/flamegraph.txt | inferno-flamegraph > build/fib.svg
# mode=full tx-file=none
	$(CKB-DEBUGGER) --bin build/fib arg1 arg2 arg3
