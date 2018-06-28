use termion;
use cpu::Cpu;
use std::fs;
use std::thread;
use std::time::Duration;
use std::io;
use std::io::{Read, Write};

use termion::raw::IntoRawMode;
use termion::async_stdin;

fn print_debug_info(cpu: &Cpu) {
    for (i, v) in cpu.v_reg.iter().enumerate() {
        print!("{}", termion::cursor::Goto(1, (i + 1) as u16));
        print!("V{} = {:X}", i, v);
    }

    print!("{}", termion::cursor::Goto(1, 17));
    print!("I = {:X}", cpu.i_reg);
    print!("{}", termion::cursor::Goto(1, 18));
    print!("PC = {:X}", cpu.prog_counter);
}

fn load_cpu_from_program_file(filepath: Option<&String>) -> (Cpu, bool) {
    let mut cpu = Cpu::new();
    let mut example = false;

    // load program into RAM
    let bytes = match filepath {
        Some(filepath) => fs::read(filepath).unwrap(),
        None => {
            example = true;
            vec![0x62, 0x01, 0xF2, 0x1E, 0x12, 0x00]
        } // example program
    };
    cpu.write_bytes(0x200, bytes.as_slice());
    cpu.prog_counter = 0x200;
    (cpu, example)
}

pub fn run_terminal(filepath: Option<&String>) {
    let term_size = termion::terminal_size().unwrap();
    if term_size.0 < 64 || term_size.1 < 32 {
        eprintln!("window size needs to be at least 64x32");
        return
    }

    let (mut cpu, example_program) = load_cpu_from_program_file(filepath);

    let stdout = io::stdout();
    let mut stdout = stdout.lock().into_raw_mode().unwrap();
    let mut stdin = async_stdin().bytes();

    print!("{}", termion::cursor::Hide);

    if example_program {
        print!("{}", termion::clear::All);
        println!("No file specified, running example program");
        thread::sleep(Duration::from_secs(2));
    }

    loop {
        print!("{}", termion::clear::All);
        print_debug_info(&cpu);
        cpu.tick();

        if !cpu.running {
            // wait for keypress
            cpu.press_key(0x2);
        }

        let b = stdin.next();
        match b {
            // q to quit
            Some(Ok(b'q')) => break,
            _ => {}
        }

        stdout.flush().unwrap();
        thread::sleep(Duration::from_millis(100));
    }
    print!("{}", termion::cursor::Show);
}