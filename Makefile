build: man
	cargo build --release

man: wsf.1
wsf.1:
	a2x --doctype manpage --format manpage README.adoc

clean:
	cargo clean
	rm -f wsf.1
