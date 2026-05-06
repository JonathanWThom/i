.PHONY: help fmt fmt-check lint test ci dev clean rev

help:
	@echo "make fmt        — apply rustfmt"
	@echo "make fmt-check  — verify rustfmt is clean (CI-friendly)"
	@echo "make lint       — clippy with warnings as errors"
	@echo "make test       — cargo test"
	@echo "make ci         — fmt-check + lint + test (mirrors GitHub Actions)"
	@echo "make dev        — alias for test"
	@echo "make rev        — cargo insta review (interactive snapshot review)"
	@echo "make clean      — cargo clean"

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test

ci: fmt-check lint test

dev: test

rev:
	cargo insta review

clean:
	cargo clean
