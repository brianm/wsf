# Washington State Ferry Schedules

wsf(1) is a cli tool to display WSF ferry schedules for the rest of the current day.

# Installation

If you are on OS X the easiest way is through [Homebrew](https://brew.sh/):

```sh
$ brew install brianm/tools/wsf
```

# Usage

See [man page](wsf.1.md)

# Building

OS X no longer ships with a working openssl, to build you'll need to
install it. Generally, follow the instructions for [rust-openssl](https://github.com/sfackler/rust-openssl#osx)
