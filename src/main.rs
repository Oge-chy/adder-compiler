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
use im::HashMap;


enum Op1 { Add1, Sub1, Negate }
enum Op2 { Plus, Minus, Times }

enum Expr {
    Number(i32),
    Id(String),
    Let(Vec<(String, Expr)>, Box<Expr>),
    UnOp(Op1, Box<Expr>),
    BinOp(Op2, Box<Expr>, Box<Expr>),
}

fn parse_expr(s: &Sexp) -> Expr {
    match s {
        Sexp::Atom(I(n)) => Expr::Number(i32::try_from(*n).expect("Invalid")),
        Sexp::Atom(S(name)) => Expr::Id(name.to_string()), 
        Sexp::List(vec) => {
            match &vec[..] {
                [Sexp::Atom(S(op)), e] if op == "add1" => Expr::UnOp(Op1::Add1, Box::new(parse_expr(e))),
                [Sexp::Atom(S(op)), e] if op == "sub1" => Expr::UnOp(Op1::Sub1, Box::new(parse_expr(e))),
                [Sexp::Atom(S(op)), e] if op == "negate" => Expr::UnOp(Op1::Negate, Box::new(parse_expr(e))),
                [Sexp::Atom(S(op)), Sexp::List(binds), body] if op == "let" => {
                    if binds.is_empty() { panic!("Invalid"); }
                    let parsed_binds = binds.iter().map(|b| parse_bind(b)).collect();
                    Expr::Let(parsed_binds, Box::new(parse_expr(body)))
                },
                [Sexp::Atom(S(op)), l, r] => {
                    let binop = match op.as_str() {
                        "+" => Op2::Plus,
                        "-" => Op2::Minus,
                        "*" => Op2::Times,
                        _ => panic!("Invalid syntax"),
                    };
                    Expr::BinOp(binop, Box::new(parse_expr(l)), Box::new(parse_expr(r)))
                },
                _ => panic!("Invalid syntax"),
            }
        },
        _ => panic!("Invalid syntax"),
    }
}

fn parse_bind(s: &Sexp) -> (String, Expr) {
    match s {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(name)), expr] => {
                if matches!(name.as_str(), "let" | "add1" | "sub1") { panic!("Invalid"); }
                (name.to_string(), parse_expr(expr))
            },
            _ => panic!("Invalid"),
        },
        _ => panic!("Invalid"),
    }
}

fn compile_to_instrs(e: &Expr, si: i32, env: &HashMap<String, i32>) -> String {
    match e {
        Expr::Number(n) => format!("  mov rax, {}", n),
        Expr::Id(s) => match env.get(s) {
            Some(offset) => format!("  mov rax, [rsp - {}]", offset),
            None => panic!("Unbound variable identifier {s}"),
        },
        Expr::UnOp(op, sub) => {
            let sub_instrs = compile_to_instrs(sub, si, env);
            let op_instr = match op {
                Op1::Add1 => "  add rax, 1",
                Op1::Sub1 => "  sub rax, 1",
                Op1::Negate => "  neg rax",
            };
            format!("{}\n{}", sub_instrs, op_instr)
        },
        Expr::BinOp(op, l, r) => {
            let l_instrs = compile_to_instrs(l, si, env);
            let stack_offset = si * 8;
            let r_instrs = compile_to_instrs(r, si + 1, env);
            let op_instrs = match op {
                Op2::Plus => format!("  add rax, [rsp - {stack_offset}]"),
                Op2::Minus => format!("  mov r10, rax\n  mov rax, [rsp - {stack_offset}]\n  sub rax, r10"),
                Op2::Times => format!("  imul rax, [rsp - {stack_offset}]"),
            };
            format!("{l_instrs}\n  mov [rsp - {stack_offset}], rax\n{r_instrs}\n{op_instrs}")
        },
        Expr::Let(binds, body) => {
            let mut curr_env = env.clone();
            let mut instrs = String::new();
            let mut curr_si = si;
            let mut seen = std::collections::HashSet::new();

            for (name, val) in binds {
                if !seen.insert(name) { panic!("Duplicate binding"); }
                instrs.push_str(&compile_to_instrs(val, curr_si, &curr_env));
                let offset = curr_si * 8;
                instrs.push_str(&format!("\n  mov [rsp - {offset}], rax\n"));
                curr_env = curr_env.update(name.to_string(), offset);
                curr_si += 1;
            }
            format!("{}{}", instrs, compile_to_instrs(body, curr_si, &curr_env))
        }
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let in_name = &args[1];
    let out_name = &args[2];

    let mut in_contents = String::new();
    File::open(in_name)?.read_to_string(&mut in_contents)?;

    let expr = parse_expr(&parse(&in_contents).expect("Invalid"));
    // Start si at 2 (offset -16) per instruction
    let result = compile_to_instrs(&expr, 2, &HashMap::new());

    let asm_program = format!("
section .text
global our_code_starts_here
our_code_starts_here:
{}
  ret", result);

    File::create(out_name)?.write_all(asm_program.as_bytes())?;
    Ok(())
}