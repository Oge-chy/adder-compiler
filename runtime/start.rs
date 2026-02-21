//create a Rust file that will call our assembly code
// The `start.rs` file is a Rust program that serves as the entry point for our assembly code. It defines an external function `our_code_starts_here` that is implemented in the assembly code, and calls it from the `main` function. The result of the assembly code is printed to the console.

#[link(name = "our_code")]
extern "C" {
    #[link_name = "\x01our_code_starts_here"]
    fn our_code_starts_here() -> i64;
}

fn main() {
    let i: i64 = unsafe {
        our_code_starts_here()
    };
    println!("{i}");
}
