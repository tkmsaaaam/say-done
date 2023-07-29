use clap::Parser;
use std::env::{self};
use std::process::Command;
use std::{thread, time};

#[derive(Parser)]
struct Args {
    #[arg(short = 'c', long = "command")]
    command: Option<String>,
    #[arg(short = 'p', long = "pid")]
    pid: Option<String>,
    #[arg(short = 't', long = "tty")]
    tty: Option<String>,
}

fn main() {
    let args = Args::parse();
    if args.command.is_none() && args.pid.is_none() && args.tty.is_none() {
        println!("arg is not present.");
        std::process::exit(0);
    }
    let target: String;
    if args.command.is_some() {
        target = args.command.clone().unwrap()
    } else if args.pid.is_some() {
        target = args.pid.clone().unwrap()
    } else {
        target = args.tty.clone().unwrap()
    }
    println!("monitoring {}", target);
    const INTERVAL: u64 = 10;
    const MAX_MONITERING_TIME: u64 = 60 * 60 * 24;
    let self_pid = std::process::id();

    for i in 0..MAX_MONITERING_TIME / INTERVAL {
        let output = Command::new("ps").output().expect("failed");
        let mut tty_count = 0;
        let mut is_continue = false;
        for process in String::from_utf8_lossy(&output.stdout).lines() {
            let process_splited: Vec<&str> = process.split_whitespace().collect();
            if process_splited[0].eq("PID") || process_splited[0].eq(&self_pid.to_string()) {
                continue;
            }

            (is_continue, tty_count) = is_found(
                args.pid.clone(),
                args.tty.clone(),
                args.command.clone(),
                process_splited,
                tty_count,
            );
            if is_continue {
                break;
            }
        }
        if is_continue {
            thread::sleep(time::Duration::from_secs(INTERVAL));
            continue;
        }

        if i == 0 {
            println!(
                "{} is not found. or {} is not started.\nps result:",
                target, target
            );
            println!("{}", String::from_utf8_lossy(&output.stdout));
            std::process::exit(0);
        }
        Command::new("say").arg("Done!").output().expect("failed");
        println!("time: {}s", i * INTERVAL);
        if env::consts::OS == "macos" {
            let arg = String::from("display notification \"")
                + &target
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
    println!("{} has been running over an hour.", &target);
}

fn is_found(
    pid: Option<String>,
    tty: Option<String>,
    command: Option<String>,
    process_splited: Vec<&str>,
    mut tty_count: i32,
) -> (bool, i32) {
    match pid {
        Some(ref pid) => {
            if process_splited[0].eq(pid) {
                return (true, tty_count);
            }
        }
        None => {}
    }

    match tty {
        Some(ref tty) => {
            if process_splited[1].eq(tty) {
                tty_count += 1;
                if tty_count > 1 {
                    return (true, tty_count);
                }
            }
        }
        None => {}
    }

    match command {
        Some(ref command) => {
            if process_splited[3].starts_with(command) {
                return (true, tty_count);
            }
        }
        None => {}
    }
    return (false, tty_count);
}
