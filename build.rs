use std::process::Command;

fn main() {
    Command::new("pandoc")
        .args(&["wsf.1.md", "-s", "-t", "man", "-o", "wsf.1"])
        .status()
        .unwrap();
}
