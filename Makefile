.PHONY: help build install release check test lint fmt clean

help:
	@echo ""
	@echo "  os-cli — Commands"
	@echo ""
	@echo "    make build      Build debug binary"
	@echo "    make install    Build release and install to ~/.local/bin/os"
	@echo "    make release    Show release instructions"
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
	mkdir -p $(HOME)/.local/bin
	cp target/release/os $(HOME)/.local/bin/os
	@echo "Installed os to ~/.local/bin/os"
	@echo "  Run: os"

release:
	@echo "To release a new version:"
	@echo "  1. Update version in Cargo.toml"
	@echo "  2. git add -A && git commit -m 'chore: bump to vX.Y.Z'"
	@echo "  3. git tag vX.Y.Z"
	@echo "  4. git push && git push --tags"
	@echo "  GitHub Actions will build and publish automatically."

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
