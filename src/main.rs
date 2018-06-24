extern crate rust_chip8;

fn main() {
    let cpu = rust_chip8::Cpu::new();
    println!("{}", cpu.delay_timer);
}
