.PHONY: dev test clippy build doc release

dev: test clippy build doc

release:
	@rustup target add x86_64-unknown-linux-musl
	@cargo build --target x86_64-unknown-linux-musl --release

test:
	@cargo test

clippy:
	@cargo clippy --all --all-features

build:
	@cargo build

doc:
	@cargo doc
