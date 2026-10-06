.PHONY: build test fmt fmt-check lint audit wasm size clean check

build:
	cargo build --workspace --all-targets --locked

test:
	cargo test --workspace --all-targets --locked

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets --locked -- -D warnings

audit:
	cargo audit

wasm:
	stellar contract build --locked --profile release

size: wasm
	@for f in target/wasm32v1-none/release/*.wasm; do echo "$$f: $$(stat -c%s "$$f") bytes"; done

# Everything CI runs, in the same order.
check: fmt-check lint test wasm

clean:
	cargo clean
