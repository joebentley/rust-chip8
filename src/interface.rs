use termion;
use cpu::Cpu;
use std::fs;
use std::thread;
use std::time::Duration;

fn print_debug_info(cpu: &Cpu) {
    for (i, v) in cpu.v_reg.iter().enumerate() {
        println!("V{} = {:X}", i, v);
    }

    println!("I = {:X}", cpu.i_reg);
    println!("PC = {:X}", cpu.prog_counter);
}

pub fn run_terminal(filepath: Option<&String>) {
    {
        let size = termion::terminal_size().unwrap();
        if size.0 < 64 || size.1 < 32 {
            eprintln!("window size needs to be at least 64x32");
            return
        }
    }

    let mut cpu = Cpu::new();

    // load program into RAM
    let f = match filepath {
        Some(filepath) => fs::read(filepath).unwrap(),
        None => vec![0x62, 0x01, 0xF2, 0x1E, 0x12, 0x00] // example program
    };
    cpu.write_bytes(0x200, f.as_slice());
    cpu.prog_counter = 0x200;

    loop {
        print!("{}", termion::clear::All);
        print_debug_info(&cpu);
        cpu.tick();

        if !cpu.running {
            // wait for keypress
            cpu.press_key(0x2);
        }

        thread::sleep(Duration::from_millis(100));
    }
}