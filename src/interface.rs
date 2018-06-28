use termion;
use cpu::Cpu;

pub fn run_terminal() {
    {
        let size = termion::terminal_size().unwrap();
        if size.0 < 64 || size.1 < 32 {
            eprintln!("window size needs to be at least 64x32");
            return
        }
    }

    let mut cpu = Cpu::new();
}