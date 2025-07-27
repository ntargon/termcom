// TermCom - Embedded Device Communication Debug Tool
mod cli;
mod tui;
mod core;
mod domain;
mod infrastructure;

fn main() {
    println!("TermCom - Embedded Device Communication Debug Tool");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
}
