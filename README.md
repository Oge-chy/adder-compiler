# Diamondback Compiler

Diamondback is a compiler written in Rust that translates a functional, Lisp-like language (S-expressions) into x86-64 assembly. This project extends previous language iterations by adding support for user-defined functions, multiple arguments, and recursive/mutually recursive function calls.

## Features Supported
* **Data Types:** 63-bit signed integers and Booleans (`true`, `false`).
* **Variables:** Local variable bindings via `let` expressions.
* **Control Flow:** `if` statements, `loop`, `break`, and `block` expressions.
* **Functions:** Top-level user-defined functions with arity checking.
* **Recursion:** Supports both direct and mutual recursion.
* **Runtime:** Includes a C/Rust runtime to handle console output, dynamic memory errors, and value untagging.

---

## Architecture: Tagging system
Because Diamondback uses a single 64-bit register to store all values, it uses a tagging system to differentiate between data types at runtime:
* **Numbers:** Shifted left by 1 bit (`n << 1`). The least significant bit (LSB) is always `0`.
* **Booleans:** `true` is represented as `3` (binary `...011`) and `false` as `1` (binary `...001`). The LSB is always `1`.

---

## x86-64 Calling Convention
This compiler implements a strict, stack-based calling convention to manage function arguments, local variables, and execution context. 

### 1. Argument Passing
Unlike the standard System V AMD64 ABI (which uses registers `rdi`, `rsi`, etc. for the first 6 arguments), Diamondback passes **all arguments on the stack**. 
* The caller evaluates arguments left-to-right.
* The caller pushes the evaluated arguments to the stack **right-to-left**.
* This ensures the first argument is always closest to the base pointer.

### 2. Stack Frame (Prologue & Epilogue)
Every function establishes its own stack frame.
* **Prologue:** ```nasm
    push rbp
    mov rbp, rsp
    sub rsp, <local_frame_size>
    ```
* **Epilogue:** ```nasm
    add rsp, <local_frame_size>
    pop rbp
    ret
    ```

### 3. Memory Layout
* **Parameters:** Accessed at positive offsets from the base pointer. Because `call` pushes the 8-byte return address, and the prologue pushes the 8-byte `rbp`, the first parameter is located at `[rbp + 16]`. Subsequent parameters are at `[rbp + 24]`, `[rbp + 32]`, etc.
* **Local Variables:** Accessed at negative offsets from the base pointer (`[rbp - 8]`, `[rbp - 16]`).

### 4. Stack Alignment & Caller Cleanup
The x86-64 architecture requires the stack pointer (`rsp`) to be **16-byte aligned** immediately before a `call` instruction. 
* If a function is called with an odd number of arguments, the compiler pads the stack by subtracting an extra 8 bytes from `rsp` prior to pushing the arguments.
* **Caller Cleanup:** Immediately after the `call` returns, the caller is responsible for shrinking the stack to remove the pushed arguments (and padding) using `add rsp, N`.

### 5. Return Values
All functions and expressions leave their final evaluated (tagged) result in the `rax` register.

---

## Interesting Programs

Here are a few examples of what can be built using the Diamondback language. You can run these by passing an initial value as a command-line argument.

### 1. Factorial
Demonstrates direct recursion and basic arithmetic.

```lisp
(fun (fact n) 
  (if (= n 0) 
      1 
      (* n (fact (- n 1)))))

(fact 5)