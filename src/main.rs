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

enum UnOp {
    Add1, Sub1, Negate,
    IsNum, IsBool,
}

enum BinOp {
    Plus, Minus, Times,
    Less, Greater, LessEq, GreaterEq, Equal,
}

enum Expr {
    Num(i32),
    Bool(bool),
    Input,
    Var(String),
    Let(Vec<(String, Expr)>, Box<Expr>),
    Set(String, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Loop(Box<Expr>),
    Break(Box<Expr>),
    Block(Vec<Expr>),
}

fn parse_expr(s: &Sexp) -> Expr {
    match s {
        Sexp::Atom(I(n)) => Expr::Num(*n as i32),
        Sexp::Atom(S(name)) if name == "true" => Expr::Bool(true),
        Sexp::Atom(S(name)) if name == "false" => Expr::Bool(false),
        Sexp::Atom(S(name)) if name == "input" => Expr::Input,
        Sexp::Atom(S(name)) => Expr::Var(name.clone()),
        

        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(op)), e] if op == "add1" => Expr::UnOp(UnOp::Add1, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "sub1" => Expr::UnOp(UnOp::Sub1, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "negate" => Expr::UnOp(UnOp::Negate, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "isnum" => Expr::UnOp(UnOp::IsNum, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "isbool" => Expr::UnOp(UnOp::IsBool, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), cond, thn, els] if op == "if" => Expr::If(Box::new(parse_expr(cond)), Box::new(parse_expr(thn)), Box::new(parse_expr(els))),
            [Sexp::Atom(S(op)), body] if op == "loop" => Expr::Loop(Box::new(parse_expr(body))),
            [Sexp::Atom(S(op)), e] if op == "break" => Expr::Break(Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), Sexp::List(binds), body] if op == "let" => {
                let parsed_binds = binds.iter().map(parse_bind).collect();
                Expr::Let(parsed_binds, Box::new(parse_expr(body)))
            }
            [Sexp::Atom(S(op)), rest @ ..] if op == "block" => {
                Expr::Block(rest.iter().map(parse_expr).collect())
            }
            [Sexp::Atom(S(op)), Sexp::Atom(S(var)), val] if op == "set!" => Expr::Set(var.clone(), Box::new(parse_expr(val))),
            [Sexp::Atom(S(op)), l, r] => {
                let binop = match op.as_str() {
                    "+" => BinOp::Plus,
                    "-" => BinOp::Minus,
                    "*" => BinOp::Times,
                    "<" => BinOp::Less,
                    ">" => BinOp::Greater,
                    "<=" => BinOp::LessEq,
                    ">=" => BinOp::GreaterEq,
                    "=" => BinOp::Equal,
                    _ => panic!("Unsupported operator"),
                };
                Expr::BinOp(binop, Box::new(parse_expr(l)), Box::new(parse_expr(r)))
            }
            _ => panic!("Invalid syntax"),
        },
        _ => panic!("Invalid syntax"),
    }
}

fn parse_bind(s: &Sexp) -> (String, Expr) {
    match s {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(name)), expr] => (name.clone(), parse_expr(expr)),
            _ => panic!("Invalid binding"),
        },
        _ => panic!("Invalid binding"),
    }
}

fn new_label(l: &mut i32, name: &str) -> String { *l += 1; format!("{}_{}", name, l) }

fn check_number() -> String {
    "  mov rbx, rax\n  and rbx, 1\n  cmp rbx, 0\n  jne error_not_number".to_string()
}

fn compile_to_instrs(
    e: &Expr,
    si: i32,
    env: &HashMap<String, i32>,
    label_counter: &mut i32,
    break_target: &Option<String>,
) -> String {
    match e {
        Expr::Num(n) => format!("  mov rax, {}", (*n as i64) << 1),
        Expr::Bool(b) => if *b { "  mov rax, 3".to_string() } else { "  mov rax, 1".to_string() },
        Expr::Var(s) => match env.get(s) { Some(offset) => format!("  mov rax, [rsp - {}]", offset), None => panic!("Unbound variable {s}"), },
        Expr::Input => "  mov rax, [rsp - 8]".to_string(),
        Expr::UnOp(op, sub) => {
            let sub_instrs = compile_to_instrs(sub, si, env, label_counter, break_target);
            let check_num = check_number();
            let op_instr = match op {
                UnOp::Add1 => "  add rax, 2",
                UnOp::Sub1 => "  sub rax, 2",
                UnOp::Negate => "  neg rax",
                UnOp::IsNum => "  and rax, 1\n  cmp rax, 0\n  sete al\n  movzb rax, al\n  shl rax, 1\n  or rax, 1",
                UnOp::IsBool => "  and rax, 1\n  cmp rax, 1\n  sete al\n  movzb rax, al\n  shl rax, 1\n  or rax, 1",
            };
            format!("{sub_instrs}\n{check_num}\n{op_instr}")
        }
        Expr::BinOp(op, l, r) => {
            let offset = si * 8;
            let l_instrs = compile_to_instrs(l, si, env, label_counter, break_target);
            let r_instrs = compile_to_instrs(r, si + 1, env, label_counter, break_target);
            let check_num = check_number();
            
            let op_instrs = match op {
                BinOp::Plus => format!("  add rax, [rsp - {offset}]"),
                BinOp::Minus => format!("  mov r10, rax\n  mov rax, [rsp - {offset}]\n  sub rax, r10"),
                BinOp::Times => format!("  mov r10, rax\n  mov rax, [rsp - {offset}]\n  imul rax, r10\n  sar rax, 1"),
                _ => {
                    let set_instr = match op {
                        BinOp::Less => "setl",
                        BinOp::Greater => "setg",
                        BinOp::LessEq => "setle",
                        BinOp::GreaterEq => "setge",
                        BinOp::Equal => "sete",
                        _ => unreachable!(),
                    };
                    format!("  cmp [rsp - {offset}], rax\n  {set_instr} al\n  movzx rax, al\n  shl rax, 1\n  or rax, 1")
                }
            };
            
            // Sequence: Compile L -> Check L -> Save L -> Compile R -> Check R -> Operation
            format!("{l_instrs}\n{check_num}\n  mov [rsp - {offset}], rax\n{r_instrs}\n{check_num}\n{op_instrs}")
        }
        Expr::If(cond, thn, els) => {

            let check_bool = "  mov rbx, rax\n  and rbx, 1\n  cmp rbx, 1\n  jne error_not_boolean"; 
            let else_l = new_label(label_counter, "if_else");
            let end_l = new_label(label_counter, "if_end");
            format!(
"{cond_code}
{check_bool}
  cmp rax, 1
  je {else_l}
{then_code}
  jmp {end_l}
{else_l}:
{else_code}
{end_l}:",
                cond_code = compile_to_instrs(cond, si, env, label_counter, break_target),
                check_bool = check_bool,
                then_code = compile_to_instrs(thn, si, env, label_counter, break_target),
                else_code = compile_to_instrs(els, si, env, label_counter, break_target),
            )
        }

        Expr::Loop(body) => {
            let start = new_label(label_counter, "loop_start");
            let end = new_label(label_counter, "loop_end");
            format!(
"{start}:
{body_code}
  jmp {start}
{end}:",
                body_code = compile_to_instrs(body, si, env, label_counter, &Some(end.clone()))
            )
        }
        Expr::Break(expr) => match break_target {
            Some(label) => format!(
"{code}
  jmp {label}",
                code = compile_to_instrs(expr, si, env, label_counter, break_target)
            ),
            None => panic!("break outside loop"),
        },
        Expr::Let(binds, body) => {
            let mut env2 = env.clone();
            let mut si2 = si;
            let mut instrs = String::new();
            for (name, val) in binds {
                let code = compile_to_instrs(val, si2, &env2, label_counter, break_target);
                let offset = si2 * 8;
                instrs.push_str(&format!("{code}\n  mov [rsp - {offset}], rax\n"));
                env2 = env2.update(name.clone(), offset);
                si2 += 1;
            }
            format!("{}{}", instrs, compile_to_instrs(body, si2, &env2, label_counter, break_target))
        }
        Expr::Set(name, val) => {
            let code = compile_to_instrs(val, si, env, label_counter, break_target);
            let offset = env.get(name).expect("set! to unbound variable");
            format!("{code}\n  mov [rsp - {offset}], rax")
        }
        Expr::Block(es) => {
            let mut instrs = String::new();
            for e in es {
                instrs.push_str(&compile_to_instrs(e, si, env, label_counter, break_target));
                instrs.push('\n');
            }
            instrs
        }

    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 { eprintln!("Usage: <input_file> <output_file>"); std::process::exit(1); }

    let in_name = &args[1];
    let out_name = &args[2];
    let mut in_contents = String::new();
    File::open(in_name)?.read_to_string(&mut in_contents)?;

    let expr = parse_expr(&parse(&in_contents).expect("Invalid Syntax"));
    let mut label_counter = 0;
    let result = compile_to_instrs(&expr, 2, &HashMap::new(), &mut label_counter, &None);

    let asm_program = format!(
"section .text
extern snek_error
global our_code_starts_here
our_code_starts_here:
  mov [rsp - 8], rdi
{result}
  ret

error_not_number:
  sub rsp, 8
  mov rdi, 1
  call snek_error
  add rsp, 8

error_not_boolean:   
  sub rsp, 8
  mov rdi, 1         
  call snek_error
  add rsp, 8",
  
        result = result
    );

    File::create(out_name)?.write_all(asm_program.as_bytes())?;
    Ok(())
}