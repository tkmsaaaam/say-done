use clap::Parser;
use std::env::{self};
use std::process::{Command, Output};
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

#[derive(Debug)]
struct Process {
    pid: String,
    tty: String,
    command: String,
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

    for i in 0..MAX_MONITERING_TIME / INTERVAL {
        let output = Command::new("ps").output().expect("failed");
        let is_continue = is_found(
            output.clone(),
            args.pid.clone(),
            args.tty.clone(),
            args.command.clone(),
        );
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

fn make_process(process: &str) -> Process {
    let process_splited: Vec<&str> = process.split_whitespace().collect();
    let mut command = String::from(process_splited[3].to_owned());
    if process_splited.len() > 4 {
        for i in 4..process_splited.len() {
            command = command + " " + process_splited[i]
        }
    }

    return Process {
        pid: process_splited[0].to_owned(),
        tty: process_splited[1].to_owned(),
        command,
    };
}

fn is_matched(
    pid: Option<String>,
    tty: Option<String>,
    command: Option<String>,
    target_process: Process,
    mut tty_count: i32,
) -> (bool, i32) {
    match pid {
        Some(ref pid) => {
            if target_process.pid.eq(pid) {
                return (true, tty_count);
            }
        }
        None => {}
    }

    match tty {
        Some(ref tty) => {
            if target_process.tty.eq(tty) {
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
            if target_process.command.starts_with(command) {
                return (true, tty_count);
            }
        }
        None => {}
    }
    return (false, tty_count);
}

fn is_found(
    output: Output,
    pid: Option<String>,
    tty: Option<String>,
    command: Option<String>,
) -> bool {
    let self_pid = std::process::id();
    let mut tty_count = 0;
    for raw_process in String::from_utf8_lossy(&output.stdout).lines() {
        if raw_process.starts_with("PID") || raw_process.starts_with(&self_pid.to_string()) {
            continue;
        }
        let process = make_process(raw_process);
        let is_continue;

        (is_continue, tty_count) = is_matched(
            pid.clone(),
            tty.clone(),
            command.clone(),
            process,
            tty_count,
        );
        if is_continue {
            return true;
        }
    }
    return false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_matched_true() {
        let pid = None;
        let tty = None;
        let command = Some(String::from("command"));
        let target_process = Process {
            pid: "00000".to_owned(),
            tty: "ttys001".to_owned(),
            command: "command".to_owned(),
        };
        let mut tty_count = 0;
        let is_continue;
        (is_continue, tty_count) = is_matched(pid, tty, command, target_process, tty_count);
        assert!(is_continue);
        assert_eq!(0, tty_count);
    }

    #[test]
    fn test_is_matched_false() {
        let pid = None;
        let tty = None;
        let command = Some(String::from("ps"));
        let target_process = Process {
            pid: "00000".to_owned(),
            tty: "ttys001".to_owned(),
            command: "command".to_owned(),
        };
        let mut tty_count = 0;
        let is_continue;
        (is_continue, tty_count) = is_matched(pid, tty, command, target_process, tty_count);
        assert!(!is_continue);
        assert_eq!(0, tty_count);
    }
}
