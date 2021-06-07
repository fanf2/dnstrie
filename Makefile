.PHONY: it cover fuzz

it:
	cargo fmt
	cargo clippy
	cargo doc --no-deps
	cargo test
	cargo run

cover:
	util/cover all

fuzz:
	cargo +nightly fuzz run all
