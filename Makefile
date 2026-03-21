.PHONY: help build install check test lint fmt clean

help:
	@echo ""
	@echo "  os-cli — Commands"
	@echo ""
	@echo "    make build      Build debug binary"
	@echo "    make install    Build release and install to /usr/local/bin/os"
	@echo "    make check      Run tests + clippy (required before commit)"
	@echo "    make test       Run tests only"
	@echo "    make lint       Run clippy only"
	@echo "    make fmt        Format code"
	@echo "    make clean      Remove build artifacts"
	@echo ""

build:
	cargo build

install:
	cargo build --release
	sudo cp target/release/os /usr/local/bin/os
	@echo "Installed os to /usr/local/bin/os"
	@echo "  Run: os"

check:
	cargo test
	cargo clippy -- -D warnings
	cargo fmt -- --check
	@echo ""
	@echo "ALL CHECKS PASSED — ready to commit"

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

clean:
	cargo clean
