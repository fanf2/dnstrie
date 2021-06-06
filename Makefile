.PHONY: it cover fuzz

it:
	cargo fmt
	cargo clippy
	cargo doc --no-deps
	cargo test
	cargo run

cover:
	util/cover bmpvec

fuzz:
	cargo +nightly fuzz run bmpvec
