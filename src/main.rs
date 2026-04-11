use im::HashMap;
use sexp::Atom::*;
use sexp::*;
use std::collections::{HashMap as StdHashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::prelude::*;

const TRUE_VAL: i64 = 3;
const FALSE_VAL: i64 = 1;
const NUM_TAG_MASK: i64 = 1;
const ERR_INVALID_ARG: i64 = 1;
const ERR_OVERFLOW: i64 = 2;
const MAX_NUM_ENC: i64 = (i32::MAX as i64) << 1;
const MIN_NUM_ENC: i64 = (i32::MIN as i64) << 1;

#[derive(Debug)]
enum Op1 {
    Add1,
    Sub1,
    Negate,
    IsNum,
    IsBool,
    Print,
}

#[derive(Debug)]
enum Op2 {
    Plus,
    Minus,
    Times,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    Equal,
}

#[derive(Debug)]
enum Expr {
    Number(i32),
    Bool(bool),
    Input,
    Id(String),
    Let(Vec<(String, Expr)>, Box<Expr>),
    UnOp(Op1, Box<Expr>),
    BinOp(Op2, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Block(Vec<Expr>),
    Loop(Box<Expr>),
    Break(Box<Expr>),
    Set(String, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug)]
struct Definition {
    name: String,
    params: Vec<String>,
    body: Expr,
}

#[derive(Debug)]
struct Program {
    defns: Vec<Definition>,
    main: Expr,
}

fn new_label(counter: &mut i32, base: &str) -> String {
    *counter += 1;
    format!("{}_{}", base, counter)
}

fn encode_num(n: i32) -> i64 {
    (n as i64) << 1
}

fn align_to_16(n: i32) -> i32 {
    if n % 16 == 0 {
        n
    } else {
        n + (16 - (n % 16))
    }
}

fn parse_program(source: &str) -> Program {
    let wrapped = format!("({})", source);
    let sexp = parse(&wrapped).unwrap_or_else(|_| panic!("Invalid"));

    match sexp {
        Sexp::List(forms) => {
            if forms.is_empty() {
                panic!("Invalid");
            }

            let mut defns = Vec::new();
            let mut main_expr: Option<Expr> = None;

            for form in forms {
                if let Some(defn) = try_parse_defn(&form) {
                    if main_expr.is_some() {
                        panic!("Invalid");
                    }
                    defns.push(defn);
                } else if main_expr.is_none() {
                    main_expr = Some(parse_expr(&form));
                } else {
                    panic!("Invalid");
                }
            }

            Program {
                defns,
                main: main_expr.unwrap_or_else(|| panic!("Invalid")),
            }
        }
        _ => panic!("Invalid"),
    }
}

fn try_parse_defn(s: &Sexp) -> Option<Definition> {
    match s {
        Sexp::List(items) => match &items[..] {
            [Sexp::Atom(S(fun_kw)), Sexp::List(signature), body] if fun_kw == "fun" => {
                match &signature[..] {
                    [Sexp::Atom(S(name)), params @ ..] => {
                        if is_reserved(name) || !is_valid_identifier(name) {
                            panic!("Invalid");
                        }

                        let mut param_names = Vec::new();
                        let mut seen = HashSet::new();
                        for p in params {
                            match p {
                                Sexp::Atom(S(param)) => {
                                    if is_reserved(param) || !is_valid_identifier(param) {
                                        panic!("Invalid");
                                    }
                                    if !seen.insert(param.clone()) {
                                        panic!("Invalid");
                                    }
                                    param_names.push(param.clone());
                                }
                                _ => panic!("Invalid"),
                            }
                        }

                        Some(Definition {
                            name: name.clone(),
                            params: param_names,
                            body: parse_expr(body),
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

fn parse_expr(s: &Sexp) -> Expr {
    match s {
        Sexp::Atom(I(n)) => Expr::Number(i32::try_from(*n).unwrap_or_else(|_| panic!("Invalid"))),
        Sexp::Atom(S(name)) => match name.as_str() {
            "true" => Expr::Bool(true),
            "false" => Expr::Bool(false),
            "input" => Expr::Input,
            _ => {
                if is_reserved(name) || !is_valid_identifier(name) {
                    panic!("Invalid");
                }
                Expr::Id(name.clone())
            }
        },
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(op)), e] if op == "add1" => Expr::UnOp(Op1::Add1, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "sub1" => Expr::UnOp(Op1::Sub1, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "negate" => Expr::UnOp(Op1::Negate, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "isnum" => Expr::UnOp(Op1::IsNum, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "isbool" => Expr::UnOp(Op1::IsBool, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "print" => Expr::UnOp(Op1::Print, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e1, e2] if op == "+" => {
                Expr::BinOp(Op2::Plus, Box::new(parse_expr(e1)), Box::new(parse_expr(e2)))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "-" => {
                Expr::BinOp(Op2::Minus, Box::new(parse_expr(e1)), Box::new(parse_expr(e2)))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "*" => {
                Expr::BinOp(Op2::Times, Box::new(parse_expr(e1)), Box::new(parse_expr(e2)))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "<" => {
                Expr::BinOp(Op2::Less, Box::new(parse_expr(e1)), Box::new(parse_expr(e2)))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == ">" => {
                Expr::BinOp(Op2::Greater, Box::new(parse_expr(e1)), Box::new(parse_expr(e2)))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "<=" => {
                Expr::BinOp(Op2::LessEq, Box::new(parse_expr(e1)), Box::new(parse_expr(e2)))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == ">=" => {
                Expr::BinOp(Op2::GreaterEq, Box::new(parse_expr(e1)), Box::new(parse_expr(e2)))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "=" => {
                Expr::BinOp(Op2::Equal, Box::new(parse_expr(e1)), Box::new(parse_expr(e2)))
            }
            [Sexp::Atom(S(op)), Sexp::List(bindings), body] if op == "let" => {
                if bindings.is_empty() {
                    panic!("Invalid");
                }
                let parsed_bindings = bindings.iter().map(parse_bind).collect();
                Expr::Let(parsed_bindings, Box::new(parse_expr(body)))
            }
            [Sexp::Atom(S(op)), cond, thn, els] if op == "if" => Expr::If(
                Box::new(parse_expr(cond)),
                Box::new(parse_expr(thn)),
                Box::new(parse_expr(els)),
            ),
            [Sexp::Atom(S(op)), exprs @ ..] if op == "block" => {
                if exprs.is_empty() {
                    panic!("Invalid");
                }
                Expr::Block(exprs.iter().map(parse_expr).collect())
            }
            [Sexp::Atom(S(op)), body] if op == "loop" => Expr::Loop(Box::new(parse_expr(body))),
            [Sexp::Atom(S(op)), e] if op == "break" => Expr::Break(Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), Sexp::Atom(S(name)), e] if op == "set!" => {
                if is_reserved(name) || !is_valid_identifier(name) {
                    panic!("Invalid");
                }
                Expr::Set(name.clone(), Box::new(parse_expr(e)))
            }
            [Sexp::Atom(S(name)), args @ ..] => {
                if is_reserved(name) || !is_valid_identifier(name) {
                    panic!("Invalid");
                }
                Expr::Call(name.clone(), args.iter().map(parse_expr).collect())
            }
            _ => panic!("Invalid"),
        },
        _ => panic!("Invalid"),
    }
}

fn parse_bind(s: &Sexp) -> (String, Expr) {
    match s {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(name)), e] => {
                if is_reserved(name) || !is_valid_identifier(name) {
                    panic!("Invalid");
                }
                (name.clone(), parse_expr(e))
            }
            _ => panic!("Invalid"),
        },
        _ => panic!("Invalid"),
    }
}

fn is_reserved(name: &str) -> bool {
    matches!(
        name,
        "let"
            | "add1"
            | "sub1"
            | "negate"
            | "isnum"
            | "isbool"
            | "print"
            | "if"
            | "block"
            | "loop"
            | "break"
            | "set!"
            | "true"
            | "false"
            | "input"
            | "fun"
    ) || matches!(name, "+" | "-" | "*" | "<" | ">" | "<=" | ">=" | "=")
}

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => (),
        _ => return false,
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn emit_check_num(lines: &mut Vec<String>, value_reg: &str) {
    lines.push(format!("mov rbx, {}", value_reg));
    lines.push(format!("and rbx, {}", NUM_TAG_MASK));
    lines.push("cmp rbx, 0".to_string());
    lines.push("jne throw_invalid_arg".to_string());
}

fn emit_check_i32_range(lines: &mut Vec<String>) {
    lines.push(format!("mov rbx, {}", MAX_NUM_ENC));
    lines.push("cmp rax, rbx".to_string());
    lines.push("jg throw_overflow".to_string());
    lines.push(format!("mov rbx, {}", MIN_NUM_ENC));
    lines.push("cmp rax, rbx".to_string());
    lines.push("jl throw_overflow".to_string());
}

fn emit_set_bool_from_cond(lines: &mut Vec<String>, jump_instr: &str, label_counter: &mut i32) {
    let true_label = new_label(label_counter, "cmp_true");
    let end_label = new_label(label_counter, "cmp_end");
    lines.push(format!("{} {}", jump_instr, true_label));
    lines.push(format!("mov rax, {}", FALSE_VAL));
    lines.push(format!("jmp {}", end_label));
    lines.push(format!("{}:", true_label));
    lines.push(format!("mov rax, {}", TRUE_VAL));
    lines.push(format!("{}:", end_label));
}

fn mem_at_rbp(offset: i32) -> String {
    if offset >= 0 {
        format!("[rbp + {}]", offset)
    } else {
        format!("[rbp - {}]", -offset)
    }
}

fn max_slot_used(e: &Expr, si: i32) -> i32 {
    match e {
        Expr::Number(_) | Expr::Bool(_) | Expr::Input | Expr::Id(_) => si - 1,
        Expr::UnOp(_, subexpr) => max_slot_used(subexpr, si),
        Expr::BinOp(_, left, right) => max_slot_used(left, si).max(max_slot_used(right, si + 1)).max(si),
        Expr::Let(bindings, body) => {
            let mut cur_si = si;
            let mut max_used = si - 1;
            for (_, bind_expr) in bindings {
                max_used = max_used.max(max_slot_used(bind_expr, cur_si)).max(cur_si);
                cur_si += 1;
            }
            max_used.max(max_slot_used(body, cur_si))
        }
        Expr::If(cond, thn, els) => max_slot_used(cond, si).max(max_slot_used(thn, si)).max(max_slot_used(els, si)),
        Expr::Block(exprs) => exprs.iter().map(|expr| max_slot_used(expr, si)).max().unwrap_or(si - 1),
        Expr::Loop(body) => max_slot_used(body, si),
        Expr::Break(expr) => max_slot_used(expr, si),
        Expr::Set(_, expr) => max_slot_used(expr, si),
        Expr::Call(_, args) => args.iter().map(|arg| max_slot_used(arg, si)).max().unwrap_or(si - 1),
    }
}

fn required_stack_slots(e: &Expr) -> i32 {
    max_slot_used(e, 1).max(0)
}

fn compile_expr(
    e: &Expr,
    si: i32,
    env: &HashMap<String, i32>,
    break_target: Option<&str>,
    label_counter: &mut i32,
    fn_sigs: &StdHashMap<String, usize>,
    params: &HashSet<String>,
) -> Vec<String> {
    match e {
        Expr::Number(n) => vec![format!("mov rax, {}", encode_num(*n))],
        Expr::Bool(b) => vec![format!("mov rax, {}", if *b { TRUE_VAL } else { FALSE_VAL })],
        Expr::Input => vec!["mov rax, rdi".to_string()],
        Expr::Id(name) => {
            let offset = *env.get(name).unwrap_or_else(|| panic!("Unbound variable identifier {}", name));
            vec![format!("mov rax, {}", mem_at_rbp(offset))]
        }
        Expr::UnOp(Op1::Add1, subexpr) => {
            let mut lines = compile_expr(subexpr, si, env, break_target, label_counter, fn_sigs, params);
            emit_check_num(&mut lines, "rax");
            lines.push("add rax, 2".to_string());
            emit_check_i32_range(&mut lines);
            lines
        }
        Expr::UnOp(Op1::Sub1, subexpr) => {
            let mut lines = compile_expr(subexpr, si, env, break_target, label_counter, fn_sigs, params);
            emit_check_num(&mut lines, "rax");
            lines.push("sub rax, 2".to_string());
            emit_check_i32_range(&mut lines);
            lines
        }
        Expr::UnOp(Op1::Negate, subexpr) => {
            let mut lines = compile_expr(subexpr, si, env, break_target, label_counter, fn_sigs, params);
            emit_check_num(&mut lines, "rax");
            lines.push("neg rax".to_string());
            emit_check_i32_range(&mut lines);
            lines
        }
        Expr::UnOp(Op1::IsNum, subexpr) => {
            let mut lines = compile_expr(subexpr, si, env, break_target, label_counter, fn_sigs, params);
            lines.push("and rax, 1".to_string());
            lines.push("cmp rax, 0".to_string());
            emit_set_bool_from_cond(&mut lines, "je", label_counter);
            lines
        }
        Expr::UnOp(Op1::IsBool, subexpr) => {
            let mut lines = compile_expr(subexpr, si, env, break_target, label_counter, fn_sigs, params);
            lines.push("and rax, 1".to_string());
            lines.push("cmp rax, 1".to_string());
            emit_set_bool_from_cond(&mut lines, "je", label_counter);
            lines
        }
        Expr::UnOp(Op1::Print, subexpr) => {
            let mut lines = compile_expr(subexpr, si, env, break_target, label_counter, fn_sigs, params);
            lines.push("mov rdi, rax".to_string());
            lines.push("call snek_print".to_string());
            lines
        }
        Expr::BinOp(op, left, right) => {
            let left_slot = -8 * si;
            let mut lines = compile_expr(left, si, env, break_target, label_counter, fn_sigs, params);
            lines.push(format!("mov {}, rax", mem_at_rbp(left_slot)));
            lines.extend(compile_expr(right, si + 1, env, break_target, label_counter, fn_sigs, params));

            match op {
                Op2::Plus => {
                    emit_check_num(&mut lines, "rax");
                    emit_check_num(&mut lines, &mem_at_rbp(left_slot));
                    lines.push(format!("add rax, {}", mem_at_rbp(left_slot)));
                    emit_check_i32_range(&mut lines);
                }
                Op2::Minus => {
                    emit_check_num(&mut lines, "rax");
                    emit_check_num(&mut lines, &mem_at_rbp(left_slot));
                    lines.push("mov rbx, rax".to_string());
                    lines.push(format!("mov rax, {}", mem_at_rbp(left_slot)));
                    lines.push("sub rax, rbx".to_string());
                    emit_check_i32_range(&mut lines);
                }
                Op2::Times => {
                    emit_check_num(&mut lines, "rax");
                    emit_check_num(&mut lines, &mem_at_rbp(left_slot));
                    lines.push("sar rax, 1".to_string());
                    lines.push(format!("imul rax, {}", mem_at_rbp(left_slot)));
                    emit_check_i32_range(&mut lines);
                }
                Op2::Less => {
                    emit_check_num(&mut lines, "rax");
                    emit_check_num(&mut lines, &mem_at_rbp(left_slot));
                    lines.push(format!("cmp {}, rax", mem_at_rbp(left_slot)));
                    emit_set_bool_from_cond(&mut lines, "jl", label_counter);
                }
                Op2::Greater => {
                    emit_check_num(&mut lines, "rax");
                    emit_check_num(&mut lines, &mem_at_rbp(left_slot));
                    lines.push(format!("cmp {}, rax", mem_at_rbp(left_slot)));
                    emit_set_bool_from_cond(&mut lines, "jg", label_counter);
                }
                Op2::LessEq => {
                    emit_check_num(&mut lines, "rax");
                    emit_check_num(&mut lines, &mem_at_rbp(left_slot));
                    lines.push(format!("cmp {}, rax", mem_at_rbp(left_slot)));
                    emit_set_bool_from_cond(&mut lines, "jle", label_counter);
                }
                Op2::GreaterEq => {
                    emit_check_num(&mut lines, "rax");
                    emit_check_num(&mut lines, &mem_at_rbp(left_slot));
                    lines.push(format!("cmp {}, rax", mem_at_rbp(left_slot)));
                    emit_set_bool_from_cond(&mut lines, "jge", label_counter);
                }
                Op2::Equal => {
                    lines.push(format!("mov rbx, {}", mem_at_rbp(left_slot)));
                    lines.push("xor rbx, rax".to_string());
                    lines.push("and rbx, 1".to_string());
                    lines.push("cmp rbx, 0".to_string());
                    lines.push("jne throw_invalid_arg".to_string());
                    lines.push(format!("cmp {}, rax", mem_at_rbp(left_slot)));
                    emit_set_bool_from_cond(&mut lines, "je", label_counter);
                }
            }
            lines
        }
        Expr::Let(bindings, body) => {
            let mut seen = HashSet::new();
            let mut lines = Vec::new();
            let mut current_si = si;
            let mut current_env = env.clone();

            for (name, bind_expr) in bindings {
                if !seen.insert(name.clone()) {
                    panic!("Duplicate binding");
                }
                if params.contains(name) {
                    panic!("Invalid");
                }
                let offset = -8 * current_si;
                lines.extend(compile_expr(bind_expr, current_si, &current_env, break_target, label_counter, fn_sigs, params));
                lines.push(format!("mov {}, rax", mem_at_rbp(offset)));
                current_env = current_env.update(name.clone(), offset);
                current_si += 1;
            }

            lines.extend(compile_expr(body, current_si, &current_env, break_target, label_counter, fn_sigs, params));
            lines
        }
        Expr::If(cond, thn, els) => {
            let else_label = new_label(label_counter, "if_else");
            let end_label = new_label(label_counter, "if_end");
            let mut lines = compile_expr(cond, si, env, break_target, label_counter, fn_sigs, params);
            lines.push(format!("cmp rax, {}", FALSE_VAL));
            lines.push(format!("je {}", else_label));
            lines.extend(compile_expr(thn, si, env, break_target, label_counter, fn_sigs, params));
            lines.push(format!("jmp {}", end_label));
            lines.push(format!("{}:", else_label));
            lines.extend(compile_expr(els, si, env, break_target, label_counter, fn_sigs, params));
            lines.push(format!("{}:", end_label));
            lines
        }
        Expr::Block(exprs) => {
            let mut lines = Vec::new();
            for expr in exprs {
                lines.extend(compile_expr(expr, si, env, break_target, label_counter, fn_sigs, params));
            }
            lines
        }
        Expr::Loop(body) => {
            let start_label = new_label(label_counter, "loop_start");
            let end_label = new_label(label_counter, "loop_end");
            let mut lines = vec![format!("{}:", start_label)];
            lines.extend(compile_expr(body, si, env, Some(end_label.as_str()), label_counter, fn_sigs, params));
            lines.push(format!("jmp {}", start_label));
            lines.push(format!("{}:", end_label));
            lines
        }
        Expr::Break(expr) => {
            let target = break_target.unwrap_or_else(|| panic!("break outside of loop"));
            let mut lines = compile_expr(expr, si, env, break_target, label_counter, fn_sigs, params);
            lines.push(format!("jmp {}", target));
            lines
        }
        Expr::Set(name, expr) => {
            let offset = *env.get(name).unwrap_or_else(|| panic!("Unbound variable identifier {}", name));
            let mut lines = compile_expr(expr, si, env, break_target, label_counter, fn_sigs, params);
            lines.push(format!("mov {}, rax", mem_at_rbp(offset)));
            lines
        }
        Expr::Call(name, args) => {
            let arity = fn_sigs.get(name).unwrap_or_else(|| panic!("Undefined function {}", name));
            if *arity != args.len() {
                panic!("Wrong number of arguments for {}", name);
            }

            let mut lines = Vec::new();
            let needs_pad = args.len() % 2 == 1;
            if needs_pad {
                lines.push("sub rsp, 8".to_string());
            }
            for arg in args.iter().rev() {
                lines.extend(compile_expr(arg, si, env, break_target, label_counter, fn_sigs, params));
                lines.push("push rax".to_string());
            }
            lines.push(format!("call fun_{}", name));

            let cleanup = (args.len() as i32) * 8 + if needs_pad { 8 } else { 0 };
            if cleanup > 0 {
                lines.push(format!("add rsp, {}", cleanup));
            }
            lines
        }
    }
}

fn compile_defn(defn: &Definition, label_counter: &mut i32, fn_sigs: &StdHashMap<String, usize>) -> String {
    let mut instrs = vec![format!("fun_{}:", defn.name)];
    instrs.push("push rbp".to_string());
    instrs.push("mov rbp, rsp".to_string());

    let slots = required_stack_slots(&defn.body);
    let frame_size = align_to_16(slots * 8);
    if frame_size > 0 {
        instrs.push(format!("sub rsp, {}", frame_size));
    }

    let mut env: HashMap<String, i32> = HashMap::new();
    let mut param_set = HashSet::new();
    for (i, param) in defn.params.iter().enumerate() {
        env = env.update(param.clone(), 16 + (i as i32) * 8);
        param_set.insert(param.clone());
    }

    instrs.extend(compile_expr(&defn.body, 1, &env, None, label_counter, fn_sigs, &param_set));

    if frame_size > 0 {
        instrs.push(format!("add rsp, {}", frame_size));
    }
    instrs.push("pop rbp".to_string());
    instrs.push("ret".to_string());
    instrs.join("\n  ")
}

fn compile_program(prog: &Program) -> String {
    let mut fn_sigs: StdHashMap<String, usize> = StdHashMap::new();
    for defn in &prog.defns {
        if fn_sigs.insert(defn.name.clone(), defn.params.len()).is_some() {
            panic!("Duplicate function {}", defn.name);
        }
    }

    let mut asm = vec![
        "section .text".to_string(),
        "extern snek_error".to_string(),
        "extern snek_print".to_string(),
        "global our_code_starts_here".to_string(),
    ];

    let mut label_counter = 0;
    for defn in &prog.defns {
        asm.push(compile_defn(defn, &mut label_counter, &fn_sigs));
    }

    asm.push("our_code_starts_here:".to_string());
    asm.push("push rbp".to_string());
    asm.push("mov rbp, rsp".to_string());

    let main_slots = required_stack_slots(&prog.main);
    let main_frame_size = align_to_16(main_slots * 8);
    if main_frame_size > 0 {
        asm.push(format!("sub rsp, {}", main_frame_size));
    }

    let env: HashMap<String, i32> = HashMap::new();
    let param_set = HashSet::new();
    asm.extend(compile_expr(&prog.main, 1, &env, None, &mut label_counter, &fn_sigs, &param_set));

    if main_frame_size > 0 {
        asm.push(format!("add rsp, {}", main_frame_size));
    }
    asm.push("pop rbp".to_string());
    asm.push("ret".to_string());

    asm.push("throw_invalid_arg:".to_string());
    asm.push(format!("mov rdi, {}", ERR_INVALID_ARG));
    asm.push("call snek_error".to_string());
    asm.push("throw_overflow:".to_string());
    asm.push(format!("mov rdi, {}", ERR_OVERFLOW));
    asm.push("call snek_error".to_string());

    format!("{}\n", asm.join("\n  "))
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input.snek> <output.s>", args[0]);
        std::process::exit(1);
    }

    let in_name = &args[1];
    let out_name = &args[2];

    let mut in_file = File::open(in_name)?;
    let mut in_contents = String::new();
    in_file.read_to_string(&mut in_contents)?;

    let program = parse_program(&in_contents);
    let asm_program = compile_program(&program);

    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_program_with_fun() {
        let p = parse_program("(fun (double x) (+ x x)) (double 5)");
        assert_eq!(p.defns.len(), 1);
    }

    #[test]
    fn parse_call_expr() {
        match parse_expr(&parse("(f 1 true)").unwrap()) {
            Expr::Call(name, args) => {
                assert_eq!(name, "f");
                assert_eq!(args.len(), 2);
            }
            other => panic!("Unexpected: {:?}", other),
        }
    }

    #[test]
    #[should_panic(expected = "Invalid")]
    fn reject_duplicate_params() {
        let _ = parse_program("(fun (f x x) x) 1");
    }

    #[test]
    #[should_panic(expected = "Undefined function")]
    fn reject_undefined_function() {
        let p = parse_program("(f 1)");
        let _ = compile_program(&p);
    }

    #[test]
    #[should_panic(expected = "Wrong number of arguments")]
    fn reject_wrong_arity() {
        let p = parse_program("(fun (f x) x) (f 1 2)");
        let _ = compile_program(&p);
    }
}
