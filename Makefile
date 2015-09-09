build: man
	cargo build --release

man: wsf.1
wsf.1:	
	pandoc -Vdate="$(shell date +'%B %Y')" README.md -s -t man > wsf.1
clean:
	cargo clean
	rm -f wsf.1
