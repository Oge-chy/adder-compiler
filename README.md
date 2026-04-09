# Cobra: A Type-Safe Expression Compiler

## Overview
Cobra is a compiler for a sophisticated subset of S-expressions, targeting x86-64 assembly. Unlike its predecessor Adder, Cobra introduces dynamic type checking, boolean logic, and variable mutation. It uses a bit-tagging scheme to distinguish between different data types at runtime.

## Features
* **Data Types:** Supports 31-bit signed integers and booleans.

* **Tagging Scheme**:
    * **Integers**: Stored as $n \times 2$ (shifted left by 1). The Least Significant Bit (LSB) is always 0.Booleans: Stored with a 2-bit pattern where the LSB is always 1.false is represented as 1 (0b01).true is represented as 3 (0b11).

    * **Runtime Checks**: Before arithmetic, we and the register with 1; if the result is not 0, we jump to error_not_number. For if conditions, we ensure the LSB is 1.


* **Arithmetic:** +, -, *, add1, sub1, negate.

* **Comparisons & Logic:** <, >, <=, >=, =, isnum, isbool.

* **Control Flow:**

    * **if:** Conditional branching based on boolean values.

    * **loop / break:** Basic iteration with exit values.

    * **block** Sequencing multiple expressions, returning the result of the last one.

* **State & Input:**

    * **let:** Local variable binding with support for shadowing.

    * **set!**: Mutation of existing variable bindings.

* **input:** Accesses the command-line argument passed to the program.

## Runtime Type Checking
Cobra ensures type safety by inspecting value tags before performing operations .Arithmetic operations and if conditions will trigger a runtime error if they receive an unexpected type.Error handling is managed by a Rust-based runtime (start.rs) which prints invalid argument and exits cleanly upon detecting a type mismatch.

## Project Structure
* **`src/main.rs`**: The core compiler logic, including the S-expression parser and the x86-64 code generator.
* **`runtime/start.rs`**: A Rust entry point that calls the compiled assembly code and prints the result.
* **`Makefile`**: Orchestrates the build process (Compiling -> Assembling -> Archiving -> Linking).
* **`test/`**: A directory containing `.snek` source files and generated assembly/binary files.

## Requirements
* **Rust & Cargo**: For building the compiler and the runtime.
* **NASM**: Netwide Assembler to process the generated `.s` files.
* **GCC/Linker**: To link the object files into an executable.
* **Environment**: Linux, macOS, or Windows with WSL.

## Usage



### Building a Program
To compile and build a specific test file (e.g., `37.snek`):
```bash
make test/37.run

