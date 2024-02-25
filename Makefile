.PHONY: fmt
fmt:
	cargo +nightly fmt --all

.PHONY: clippy
clippy:
	cargo clippy --locked --workspace  -- -D warnings

.PHONY: lint
lint: fmt clippy