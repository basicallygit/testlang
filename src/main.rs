use std::process;
use std::fs::{File, read_to_string};
use std::path::Path;
use std::io::{Write, BufWriter};
use std::env;

#[allow(non_camel_case_types)]
enum Operation {
    OP_PUSH,
    OP_ADD,
    OP_SUB,
    OP_MUL,
    OP_DIV,
    OP_EQ,
    OP_DUMP,
    OP_IF,
    OP_ELSE,
    OP_END,
    OP_NOP,
}

fn generate_asm(file: impl AsRef<Path>, code: Vec<(Operation, Option<&str>)>) -> std::io::Result<()> {
    let dump = "dump:
    mov     r8, -3689348814741910323
    sub     rsp, 40
    mov     BYTE [rsp+31], 10
    lea     rcx, [rsp+30]
.L2:
    mov     rax, rdi
    mul     r8
    mov     rax, rdi
    shr     rdx, 3
    lea     rsi, [rdx+rdx*4]
    add     rsi, rsi
    sub     rax, rsi
    mov     rsi, rcx
    sub     rcx, 1
    add     eax, 48
    mov     BYTE [rcx+1], al
    mov     rax, rdi
    mov     rdi, rdx
    cmp     rax, 9
    ja      .L2
    lea     rdx, [rsp+32]
    mov     edi, 1
    sub     rdx, rsi
    mov     rax, 1
    syscall
    add     rsp, 40
    ret\n\n";
    let mut file = BufWriter::new(File::create(file)?);
    file.write_all("segment .text\n".as_bytes())?;
    file.write_all(dump.as_bytes())?;
    file.write_all("global _start\n_start:\n".as_bytes())?;

    let i = 0;
    for (op, arg) in code {
        match op {
            Operation::OP_PUSH => {
                file.write_all(format!("    push {}\n", arg.unwrap()).as_bytes())?;
            }
            Operation::OP_ADD => {
                file.write_all("    pop rax\n    pop rbx\n    add rax, rbx\n    push rax\n".as_bytes())?;
            }
            Operation::OP_SUB => {
                file.write_all("    pop rax\n    pop rbx\n    sub rbx, rax\n    push rbx\n".as_bytes())?;
            }
            Operation::OP_MUL => {
                file.write_all("    pop rax\n    pop rbx\n    imul rbx, rax\n    push rbx\n".as_bytes())?;
            }
            Operation::OP_DIV => {
                file.write_all("    pop rax\n    pop rbx\n    cqo\n    idiv rbx\n    push rax\n".as_bytes())?;
            }
            Operation::OP_DUMP => {
                file.write_all("    pop rdi\n    call dump\n".as_bytes())?;
            }
            Operation::OP_EQ => {
                file.write_all("    mov rcx, 0\n    mov rdx, 1\n    pop rax\n    pop rbx\n    cmp rax, rbx\n    cmove rcx, rdx\n    push rcx\n".as_bytes())?;
            }
            Operation::OP_IF => {
                file.write_all("    pop rax\n    test rax, rax\n    jz .addr_".as_bytes())?;
                file.write_all((i+1).to_string().as_bytes())?;
                file.write_all("\n".as_bytes())?;
            }
            Operation::OP_ELSE => {
                file.write_all(format!("    jmp .addr_{}\n", i).as_bytes())?;
                file.write_all(format!("\n.addr_{}:\n", i+1).as_bytes())?;
            }
            Operation::OP_END => {
                file.write_all("\n.addr_".as_bytes())?;
                file.write_all(i.to_string().as_bytes())?;
                file.write_all(":\n".as_bytes())?;
            }
            Operation::OP_NOP => {
                file.write_all("    nop\n".as_bytes())?;
            }
        }
    }
    file.write_all("    mov rax, 60\n    mov rdi, 0\n    syscall\n".as_bytes())?;

    Ok(())
}

fn parse_word_to_op(word: &str) -> (Operation, Option<&str>) {
    match word {
        "+" => (Operation::OP_ADD, None),
        "-" => (Operation::OP_SUB, None),
        "." => (Operation::OP_DUMP, None),
        "=" => (Operation::OP_EQ, None),
        "*" => (Operation::OP_MUL, None),
        "/" => (Operation::OP_DIV, None),
        "if" => (Operation::OP_IF, None),
        "else" => (Operation::OP_ELSE, None),
        "end" => (Operation::OP_END, None),
        "nop" => (Operation::OP_NOP, None),
        _ => (Operation::OP_PUSH, Some(word)),
    }
}

fn main() -> std::io::Result<()> {
    if cfg!(windows) {
        eprintln!("testlang is not supported on windows yet, please try WSL or a VM.");
        process::exit(1);
    }
    let mut autorun = false;

    let args = env::args().collect::<Vec<String>>();
    if args.contains(&String::from("-r")) {
        autorun = true;
    }
    if args.len() < 2 {
        eprintln!("Usage: {} <file> [-r]", args[0]);
        process::exit(1);
    }

    let mut code = Vec::new();
    let file = read_to_string(&args[1])?;
    for line in file.lines().map(|line| line.split_whitespace().collect::<Vec<&str>>()) {
        if line.len() == 0 {continue;}
        for word in line {
            code.push(parse_word_to_op(word));
        }
    }

    println!("[+] Generating assembly...");
    generate_asm("output.asm", code)?;

    let mut output;
    println!("[+] Compiling with nasm...");
    //wait for nasm to compile
    output = process::Command::new("nasm")
        .arg("-f")
        .arg("elf64")
        .arg("output.asm")
        .arg("-o")
        .arg("output.o")
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        eprintln!("nasm failed to compile, output:");
        eprintln!("stout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("exit code: {}", output.status.code().unwrap());
        process::exit(1);
    }
    println!("[+] Linking with ld...");
    output = process::Command::new("ld")
        .arg("-o")
        .arg("output")
        .arg("output.o")
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        eprintln!("ld failed to link, output:");
        eprintln!("stout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("exit code: {}", output.status.code().unwrap());
        process::exit(1);
    }

    if autorun {
        println!("[+] Running...");
        output = process::Command::new("./output")
            .output()
            .expect("failed to execute process");
        println!("\n{}", String::from_utf8_lossy(&output.stdout));
    }

    println!("[+] Done!");

    Ok(())
}
