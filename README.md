# Adder: A Simple Expression Compiler

## Overview
Compiler for the **Adder** language, a minimal language supporting 32-bit signed integers and unary operations. It translates concrete S-expression syntax into x86-64 assembly instructions, which are then assembled and linked with a Rust runtime.

## Features
* **Integer Support:** Handles 32-bit signed integers (from -2147483648 to 2147483647).
* **Unary Operations:**
    * `add1`: Increments the result of an expression by 1.
    * `sub1`: Decrements the result of an expression by 1.
    * `negate`: Multiplies the result of an expression by -1.
* **Recursive Parsing:** Supports nested expressions like `(add1 (sub1 (add1 73)))`.



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

