extern crate rust_chip8;
use std::env;

fn main() {
    let args : Vec<_> = env::args().collect();
    rust_chip8::parse_args_and_run_terminal(args);
}
