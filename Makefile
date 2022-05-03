
.PHONY: build
build: 
	@echo "use cargo to build, this is here only for the manpage (make man)"

.PHONY: man
man:
	pandoc wsf.1.md -s -t man -o wsf.1

