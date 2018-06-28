extern crate rand;
extern crate termion;

pub mod utils;
pub use utils::get_nth_hex_digit;

pub mod cpu;
pub use cpu::Cpu;

pub mod interface;
pub use interface::run_terminal;