use termion;
use cpu::{Cpu, Display};
use std::fs;
use std::thread;
use std::time::Duration;
use std::io;
use std::io::{Read, Write};
use std::path::Path;

use termion::raw::IntoRawMode;
use termion::async_stdin;

fn draw_screen(display: &Display) {
    for (y, row) in display.pixels.iter().enumerate() {
        print!("{}", termion::cursor::Goto(1, (y + 1) as u16));

        let mut display_row = String::new();

        for i in (0..64).rev() {
            let mask = 1 << i;
            let filled = (row & mask) >> i != 0;
            if filled {
                display_row += "x";
            } else {
                display_row += ".";
            }
        }

        print!("{}", display_row);
    }
}

fn print_debug_info(cpu: &Cpu, program_name: &str) {
    for (i, v) in cpu.v_reg.iter().enumerate() {
        print!("{}", termion::cursor::Goto(1, (i + 1) as u16));
        print!("V{} = {:X}", i, v);
    }

    print!("{}", termion::cursor::Goto(1, 17));
    print!("I = {:X}", cpu.i_reg);
    print!("{}", termion::cursor::Goto(1, 18));
    print!("PC = {:X}", cpu.prog_counter);
    print!("{}", termion::cursor::Goto(1, 19));
    print!("SP = {:X}", cpu.stack_pointer);
    print!("{}", termion::cursor::Goto(1, 20));
    print!("prog name = {}", program_name);

    print!("{}", termion::cursor::Goto(1, 24));
    print!("press q to exit");
}

fn load_cpu_from_program_file(filepath: Option<&str>) -> (Cpu, bool) {
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

pub fn run_terminal(filepath: Option<&str>, debug_mode: bool) {
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

    let program_name = match filepath {
        Some(path) => Path::new(path).file_stem().unwrap().to_str().unwrap(),
        None => "Example"
    };

    loop {
        print!("{}", termion::clear::All);

        if example_program || debug_mode {
            print_debug_info(&cpu, program_name);
        } else {
            draw_screen(&cpu.display);
        }

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

pub fn parse_args_and_run_terminal(args: Vec<String>) {
    let mut debug = false;
    let mut filepath = None;

    for (i, arg) in args.iter().enumerate() {
        if arg == "-d" {
            debug = true;
        }

        if arg == "-f" {
            match args.get(i + 1) {
                Some(arg) => filepath = Some(arg.as_str()),
                None => {}
            }
        }
    }

    run_terminal(filepath, debug);
}