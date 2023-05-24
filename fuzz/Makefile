fuzz:
	cargo afl build --release
	mkdir -p in out
	echo 'aaaaaaa' > in/foo
	cargo afl fuzz -i in -o out target/release/fuzz
.PHONY: fuzz
