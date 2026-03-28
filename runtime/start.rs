// start.rs - Rust entry point for our assembly code

use std::env;

#[link(name = "our_code")]
extern "C" {
    // Assembly function: accepts one input argument (tagged i64) and returns a tagged i64
    #[link_name = "\x01our_code_starts_here"]
    fn our_code_starts_here(input: i64) -> i64;
}

#[no_mangle]
pub extern "C" fn snek_error(errcode: i64) {
    match errcode {
        1 => eprintln!("invalid argument"), // Type mismatch error
        2 => eprintln!("overflow"),         // Arithmetic overflow
        _ => eprintln!("unknown error code: {}", errcode),
    }
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse optional input from command line; default to 'false' (1) if not provided
    let input_val: i64 = if args.len() > 1 {
        let parsed = args[1].parse::<i64>().expect("Invalid input number");
        parsed << 1 // Tag the input as a number (LSB = 0)
    } else {
        1 // Default to 'false' if no input provided
    };

    // Call the assembly code
    let result: i64 = unsafe { our_code_starts_here(input_val) };

    // --- Tagged Printing Logic ---
    match result {
        1 => println!("false"),      // Tagged false
        3 => println!("true"),       // Tagged true
        n if n & 1 == 0 => println!("{}", n >> 1), // Number: untag and print
        _ => println!("Unknown value: {}", result),
    }
}