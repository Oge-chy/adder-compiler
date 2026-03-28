// start.rs - Rust entry point for our assembly code

use std::env;

#[link(name = "our_code")]
extern "C" {
    #[link_name = "\x01our_code_starts_here"]
    fn our_code_starts_here(input: i64) -> i64;
}

#[no_mangle]
pub extern "C" fn snek_error(errcode: i64) {
    if errcode == 1 { eprintln!("invalid argument"); }
    else if errcode == 2 { eprintln!("overflow"); }
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_val = if args.len() > 1 { args[1].parse::<i64>().expect("Invalid input") << 1 } else { 1 };
    let result: i64 = unsafe { our_code_starts_here(input_val) };

    if result & 1 == 0 { println!("{}", result >> 1); }
    else if result == 3 { println!("true"); }
    else if result == 1 { println!("false"); }
    else { println!("Unknown value: {result}"); }
}