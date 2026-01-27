.PHONY: all build test bench clean

SO = target/release/libleak_alloc.so

all: build

build:
	cargo build --release

test: build test.c
	gcc -o test test.c
	LD_PRELOAD=$(SO) ./test

bench: build bench.c
	gcc -O2 -pthread -o bench bench.c
	@echo "=== System allocator ==="
	./bench
	@echo ""
	@echo "=== Leak allocator ==="
	LD_PRELOAD=$(SO) ./bench

clean:
	cargo clean
	rm -f test bench
