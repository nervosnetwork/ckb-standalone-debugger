TARGET := riscv64-unknown-linux-gnu
CC := $(TARGET)-gcc
LD := $(TARGET)-gcc
OBJCOPY := $(TARGET)-objcopy
# copy ckb-c-stdlib here before continue
CFLAGS := -O0 -fno-builtin-printf -nostdinc \
-nostdlib -nostartfiles -I ./ckb-c-stdlib/libc -I ./ckb-c-stdlib -g
LDFLAGS := -Wl,-static
# docker pull nervos/ckb-riscv-gnu-toolchain:gnu-bionic-20191012
BUILDER_DOCKER := nervos/ckb-riscv-gnu-toolchain@sha256:aae8a3f79705f67d505d1f1d5ddc694a4fd537ed1c7e9622420a470d59ba2ec3

CKB-DEBUGGER := ../target/debug/ckb-debugger
all: build/fib build/outofmemory

all-via-docker:
	docker run --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make"

build/fib: fib.c
	mkdir -p build
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

build/outofmemory: outofmemory.c
	mkdir -p build
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<

clean:
	rm build/fib
	rm build/output.txt
	rm build/fib.svg

run:
# mode=full
	$(CKB-DEBUGGER) --tx-file=../../tests/programs/sample.json --script-group-type=type \
		--script-hash=0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c --script-version=0
	$(CKB-DEBUGGER) --tx-file=../../tests/programs/sample_data1.json --script-group-type=type \
		--script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
# mode=full pprof=build/flamegraph.txt
	$(CKB-DEBUGGER) --pprof build/flamegraph.txt --tx-file=../../tests/programs/sample.json \
		--script-group-type=type --script-hash=0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c --script-version=0
	$(CKB-DEBUGGER) --pprof build/flamegraph.txt --tx-file=../../tests/programs/sample_data1.json \
		--script-group-type=type --script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
# mode=fast
	$(CKB-DEBUGGER) --mode fast --tx-file=../../tests/programs/sample.json --script-group-type=type \
		--script-hash=0xee75995da2e55e6c4938533d341597bc10add3837cfe57174f2ee755da82555c --script-version=0
	$(CKB-DEBUGGER) --mode fast --tx-file=../../tests/programs/sample_data1.json --script-group-type=type \
		--script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
# mode=full pprof=build/flamegraph.txt
	RUST_LOG=trace $(CKB-DEBUGGER) --bin=build/fib --pprof=build/flamegraph.txt \
		--tx-file=../../tests/programs/sample_data1.json --script-group-type=type \
		--script-hash=0xca505bee92c34ac4522d15da2c91f0e4060e4540f90a28d7202df8fe8ce930ba
	cat build/flamegraph.txt | inferno-flamegraph > build/fib.svg
# mode=full tx-file=none
	$(CKB-DEBUGGER) --bin build/fib arg1 arg2 arg3
