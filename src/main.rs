// A simple compiler from a subset of S-expressions to x86-64 assembly.
// The input language is defined by the `Expr` enum, and the output is a string of assembly code that computes the value of the expression and leaves it in the `rax` register.
// The compiler is implemented in the `compile_expr` function, which recursively compiles sub-expressions and combines their results using the appropriate assembly instructions.
// The `main` function reads an input file containing an S-expression, parses it into an `Expr`, compiles it to assembly, and writes the assembly code to an output file.
// The `Makefile` defines how to build the assembly code and the Rust program that calls it, using `nasm` to assemble the code and `rustc` to compile the Rust program.

use sexp::*;
use sexp::Atom::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;

enum Expr {
    Num(i32),
    Add1(Box<Expr>),
    Sub1(Box<Expr>),
    Negate(Box<Expr>),
}

fn parse_expr(s: &Sexp) -> Expr {
    match s {
        Sexp::Atom(I(n)) => Expr::Num(i32::try_from(*n).unwrap()),
        Sexp::List(vec) => {
            match &vec[..] {
                [Sexp::Atom(S(op)), e] if op == "add1" => 
                    Expr::Add1(Box::new(parse_expr(e))),
                [Sexp::Atom(S(op)), e] if op == "sub1" => 
                    Expr::Sub1(Box::new(parse_expr(e))),
                // TODO: Add negate case
                [Sexp::Atom(S(op)), e] if op == "negate" => 
                    Expr::Negate(Box::new(parse_expr(e))),
                _ => panic!("Invalid expression"),
            }
        },
        _ => panic!("Invalid expression"),
    }
}

fn compile_expr(e: &Expr) -> String {
    match e {
        Expr::Num(n) => format!("mov rax, {}", *n),
        Expr::Add1(subexpr) => compile_expr(subexpr) + "\n  add rax, 1",
        Expr::Sub1(subexpr) => compile_expr(subexpr) + "\n  sub rax, 1",
        // TODO: Add negate case
        Expr::Negate(subexpr) => compile_expr(subexpr) + "\n  neg rax", 
    }
}



fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let in_name = &args[1];
    let out_name = &args[2];

    let mut in_file = File::open(in_name)?;
    let mut in_contents = String::new();
    in_file.read_to_string(&mut in_contents)?;

    let expr = parse_expr(&parse(&in_contents).unwrap());
    let result = compile_expr(&expr);
    
    let asm_program = format!("
section .text
global our_code_starts_here
our_code_starts_here:
    {}
    ret
", result);

    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;

    Ok(())
}