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

fn draw_screen_terminal(display: &Display) {
    for (y, row) in display.pixels.iter().enumerate() {

        let mut display_row = String::new();

        for i in (0..64).rev() {
            let mask = 1 << i;
            let filled = (row & mask) >> i != 0;
            if filled {
                print!("{}", termion::cursor::Goto(1, (y + 1) as u16));
                display_row += "x";
            } else {
                display_row += " ";
            }
        }

        print!("{}", display_row);
    }
}

fn print_debug_info_terminal(cpu: &Cpu, program_name: &str) {
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

pub fn run(filepath: Option<&str>, debug_mode: bool) {
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
            print_debug_info_terminal(&cpu, program_name);
        } else {
            draw_screen_terminal(&cpu.display);
        }

        cpu.tick();

        if !cpu.running {
            // wait for keypress
            cpu.press_key(0x2);
        }

        let b = stdin.next();
        match b {
            // ; to quit
            Some(Ok(b';')) => break,

            // key map
            Some(Ok(b'1')) => cpu.press_key(0x1),
            Some(Ok(b'2')) => cpu.press_key(0x2),
            Some(Ok(b'3')) => cpu.press_key(0x3),
            Some(Ok(b'4')) => cpu.press_key(0xC),
            Some(Ok(b'q')) => cpu.press_key(0x4),
            Some(Ok(b'w')) => cpu.press_key(0x5),
            Some(Ok(b'e')) => cpu.press_key(0x6),
            Some(Ok(b'r')) => cpu.press_key(0xD),
            Some(Ok(b'a')) => cpu.press_key(0x7),
            Some(Ok(b's')) => cpu.press_key(0x8),
            Some(Ok(b'd')) => cpu.press_key(0x9),
            Some(Ok(b'f')) => cpu.press_key(0xE),
            Some(Ok(b'z')) => cpu.press_key(0xA),
            Some(Ok(b'x')) => cpu.press_key(0x0),
            Some(Ok(b'c')) => cpu.press_key(0xB),
            Some(Ok(b'v')) => cpu.press_key(0xF),
            _ => true
        };

        stdout.flush().unwrap();
        thread::sleep(Duration::from_millis(1));
    }

    print!("{}", termion::cursor::Show);
}
