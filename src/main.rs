use std::env;
use std::io::{self, Write};
use std::process::Command;

fn main() {
    loop {
        let cwd = env::current_dir().unwrap_or_default(); //current working dir
        let last = cwd
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("/");
        print!("nsh:{} ~ ", last);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        if input.is_empty() {
            continue;
        }

        let mut parts = input.split_whitespace();
        let program = parts.next().unwrap();
        let args: Vec<&str> = parts.collect();

        match program {
            "exit" | "quit" => {
                println!("Quiting nsh!!");
            }
            "cd" => {
                let new_dir = args.first().copied().unwrap_or("/");
                if let Err(e) = env::set_current_dir(new_dir) {
                    eprintln!("cd error: {}", e);
                }
            }
            _ => {
                let child = Command::new(program).args(args).spawn();

                match child {
                    Ok(mut child_process) => {
                        child_process.wait().unwrap();
                    }
                    Err(e) => {
                        eprintln!(
                            "nsh: command not found or execution error: {} - {}",
                            program, e
                        );
                    }
                }
            }
        }
    }
}
