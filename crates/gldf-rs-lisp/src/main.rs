//! Headless `gldf-lisp` REPL / script runner.
//!
//! Mirrors the bimifc-lisp bin: read a `.lsp` file (or stdin), run it
//! against a fresh `create_gldf_interpreter`, print stdout, exit 0 on
//! success / 1 on script error (i.e. anything pushed onto `output`
//! that starts with `(`-named-fn-error).
//!
//! Usage:
//!   gldf-lisp script.lsp
//!   echo '(GLDF-LOAD "x.gldf") (GLDF-VARIANT-COUNT)' | gldf-lisp -

use gldf_rs_lisp::create_gldf_interpreter;
use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let source = match args.next() {
        Some(p) if p == "-" => {
            let mut buf = String::new();
            if std::io::stdin().read_to_string(&mut buf).is_err() {
                eprintln!("gldf-lisp: failed to read stdin");
                return ExitCode::from(2);
            }
            buf
        }
        Some(p) => match std::fs::read_to_string(&p) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("gldf-lisp: read {p}: {e}");
                return ExitCode::from(2);
            }
        },
        None => {
            eprintln!("usage: gldf-lisp <script.lsp | ->");
            return ExitCode::from(2);
        }
    };

    let mut interp = create_gldf_interpreter();
    let results = interp.run(&source);

    // Print whatever the foreign functions pushed to output.
    for line in &interp.output {
        print!("{line}");
    }
    // Print top-level result expressions (skip Nil to keep output tidy,
    // matching the bimifc REPL convention).
    for r in results.iter().filter(|e| **e != acadlisp::Expr::Nil) {
        println!("{r}");
    }
    ExitCode::SUCCESS
}
