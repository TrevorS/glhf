.PHONY: build release install clean test lint fmt check

build:
	cargo build

release:
	cargo build --release

install:
	cargo install --path .

clean:
	cargo clean

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

check: fmt lint test
