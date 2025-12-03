use std::process::{Command, exit};
use std::env;
use std::path::Path;

fn main() {
    // The first argument is "cargo", the second is the subcommand (e.g., "build-wasm").
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo build-wasm|build-python");
        exit(1);
    }

    match args[1].as_str() {
        "build-wasm" => build_wasm(),
        "build-python" => build_python(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Usage: cargo build-wasm|build-python");
            exit(1);
        }
    }
}

fn build_wasm() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let wasm_dir = current_dir.join("gldf-rs-wasm");

    if !Path::new(&wasm_dir).exists() {
        eprintln!("Error: gldf-rs-wasm directory does not exist.");
        exit(1);
    }

    let status = Command::new("trunk")
        .arg("build")
        .current_dir(&wasm_dir)
        .status()
        .expect("Failed to execute trunk build");

    if !status.success() {
        eprintln!("Error: Trunk build failed.");
        exit(1);
    } else {
        println!("Trunk build succeeded.");
    }
}

fn build_python() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let python_dir = current_dir.join("gldf-rs-python");

    if !Path::new(&python_dir).exists() {
        eprintln!("Error: gldf-rs-python directory does not exist.");
        exit(1);
    }

    let status = Command::new("maturin")
        .arg("develop")
        .current_dir(&python_dir)
        .status()
        .expect("Failed to execute maturin develop");

    if !status.success() {
        eprintln!("Error: Maturin develop failed.");
        exit(1);
    } else {
        println!("Maturin develop succeeded.");
    }
}
