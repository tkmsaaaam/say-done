use clap::Parser;
use std::env;
use std::process::Command;
use std::{thread, time};

#[derive(Parser)]
struct Args {
    #[arg(short = 'c', long = "command")]
    command: String,
    #[arg(short = 'p', long = "pid")]
    pid: Option<String>,
    #[arg(short = 't', long = "tty")]
    tty: Option<String>,
}

fn main() {
    let args = Args::parse();
    println!("monitoring {}", args.command);
    const INTERVAL: u64 = 10;
    const MAX_MONITERING_TIME: u64 = 60 * 60 * 24;
    let self_pid = std::process::id();

    for i in 0..MAX_MONITERING_TIME / INTERVAL {
        thread::sleep(time::Duration::from_secs(INTERVAL));
        let output = Command::new("ps").output().expect("failed");
        let mut tty_count = 0;
        for process in String::from_utf8_lossy(&output.stdout).lines() {
            let process_splited: Vec<&str> = process.split_whitespace().collect();
            if process_splited[0].eq("PID") || process_splited[0].eq(&self_pid.to_string()) {
                continue;
            }

            match args.pid {
                Some(ref pid) => {
                    if process_splited[0].eq(pid) {
                        continue;
                    }
                }
                None => {}
            }

            match args.tty {
                Some(ref tty) => {
                    if process_splited[1].eq(tty) {
                        tty_count += 1;
                        if tty_count > 1 {
                            continue;
                        }
                    }
                }
                None => {}
            }

            if process_splited[3].starts_with(&args.command) {
                continue;
            }
        }
        if i == 0 {
            println!(
                "{} is not found. or {} is not started.\nps result:",
                args.command, args.command
            );
            println!("{}", String::from_utf8_lossy(&output.stdout));
            std::process::exit(0);
        }
        Command::new("say").arg("Done!").output().expect("failed");
        println!("time: {}s", i * INTERVAL);
        if env::consts::OS == "macos" {
            let arg = String::from("display notification \"")
                + &args.command
                + " was ended.\" with title \""
                + env!("CARGO_PKG_NAME")
                + "\""; // display notification "CMD was ended." with title "CMD"
            Command::new("osascript")
                .arg("-e")
                .arg(arg)
                .output()
                .expect("failed");
        }
        std::process::exit(0);
    }
    println!("{} has been running over an hour.", &args.command);
}
