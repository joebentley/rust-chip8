
pub mod termion;
pub mod sdl;

pub fn parse_args_and_run(args: Vec<String>) {
    let mut debug = false;
    let mut filepath = None;
    let mut termion_mode = true;

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

        if arg == "--sdl" {
            termion_mode = false;
        }
    }

    if termion_mode {
        termion::run(filepath, debug);
    } else {
        sdl::run(filepath, debug);
    }
}