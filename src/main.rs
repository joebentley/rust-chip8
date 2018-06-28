extern crate rust_chip8;
use std::env;

fn main() {
    let args : Vec<_> = env::args().collect();
    rust_chip8::run_terminal(args.get(1));
}
