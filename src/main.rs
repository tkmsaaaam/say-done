use clap::Parser;
use std::collections::HashMap;
use std::env::{self};
use std::process::{Command, Output};
use std::{thread, time};

#[derive(Debug, Parser, Clone)]
struct Args {
    #[arg(short = 'c', long = "command")]
    command: Option<String>,
    #[arg(short = 'p', long = "pid")]
    pid: Option<String>,
    #[arg(short = 't', long = "tty")]
    tty: Option<String>,
}

#[derive(Clone)]
struct Process {
    pid: String,
    command: String,
}

fn main() {
    let args = match make_args() {
        Some(args) => args,
        None => std::process::exit(0),
    };
    let target = make_target(args.clone());
    println!("monitoring {}", target);
    const INTERVAL: u64 = 10;
    const MAX_MONITERING_TIME: u64 = 60 * 60 * 24;

    for i in 0..MAX_MONITERING_TIME / INTERVAL {
        let output = Command::new("ps").output().expect("ps was failed.");
        let process_map = make_process_map(output.clone());
        let is_continue = is_found(args.clone(), process_map);
        if is_continue {
            thread::sleep(time::Duration::from_secs(INTERVAL));
            continue;
        }

        if i == 0 {
            println!(
                "{} is not found. or {} is not started.\nps result:",
                target, target
            );
            println!("{:?}", String::from_utf8(output.stdout));
            std::process::exit(0);
        }
        Command::new("say")
            .arg("Done!")
            .output()
            .expect("say was failed.");
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
                .expect("osascript was failed.");
        }
        std::process::exit(0);
    }
    println!("{} has been running over an hour.", target);
}

fn make_args() -> Option<Args> {
    let args = Args::parse();

    if args.command.is_some() || args.pid.is_some() || args.tty.is_some() {
        return Some(args);
    }
    println!("args is not present.");
    println!("command:");
    let mut command = String::new();
    std::io::stdin().read_line(&mut command).expect("");

    println!("pid:");
    let mut pid = String::new();
    std::io::stdin().read_line(&mut pid).expect("");

    println!("tty:");
    let mut tty = String::new();
    std::io::stdin().read_line(&mut tty).expect("");

    if command.trim_end().is_empty() && pid.trim_end().is_empty() && tty.trim_end().is_empty() {
        return None;
    } else {
        return Some(Args {
            command: Some(String::from(command.trim_end())),
            pid: Some(String::from(pid.trim_end())),
            tty: Some(String::from(tty.trim_end())),
        });
    }
}

fn make_target(args: Args) -> String {
    let mut target = String::new();

    match args.command {
        Some(ref command) => target = target + "command: " + command + " ",
        None => {}
    }

    match args.pid {
        Some(ref pid) => target = target + "pid: " + pid + " ",
        None => {}
    }

    match args.tty {
        Some(ref tty) => target = target + "tty: " + tty + " ",
        None => {}
    }
    return target;
}

fn make_process(process: &str) -> (String, Process) {
    let process_splited: Vec<&str> = process.split_whitespace().collect();
    let pid_index = 0;
    let tty_index = 1;
    let command_start_index = 3;
    let mut command = String::from(process_splited[command_start_index]);
    if process_splited.len() > (command_start_index + 1) {
        for i in (command_start_index + 1)..process_splited.len() {
            command = command + " " + process_splited[i]
        }
    }

    return (
        String::from(process_splited[tty_index]),
        Process {
            pid: String::from(process_splited[pid_index]),
            command,
        },
    );
}

fn is_matched(args: Args, target_tty: String, target_process_list: Vec<Process>) -> bool {
    match args.pid {
        Some(ref pid) => {
            let c = target_process_list
                .iter()
                .filter(|process| process.pid.eq(pid))
                .count();
            if c > 0 {
                return true;
            }
        }
        None => {}
    }

    match args.tty {
        Some(ref tty) => {
            if target_tty.eq(tty) && target_process_list.len() > 1 {
                return true;
            }
        }
        None => {}
    }

    match args.command {
        Some(ref command) => {
            let c = target_process_list
                .iter()
                .filter(|process| process.command.starts_with(command))
                .count();
            if c > 0 {
                return true;
            }
        }
        None => {}
    }
    return false;
}

fn make_process_map(output: Output) -> HashMap<String, Vec<Process>> {
    let self_pid = std::process::id();
    let mut process_map: HashMap<String, Vec<Process>> = HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.starts_with("  PID") || line.starts_with(&self_pid.to_string()) {
            continue;
        }
        let tty: String;
        let process: Process;
        (tty, process) = make_process(line);

        match process_map.get(&tty) {
            Some(process_list) => {
                let mut new_process_list = process_list.to_vec();
                new_process_list.push(process);
                process_map.insert(tty, new_process_list);
            }
            None => {
                process_map.insert(tty, Vec::from([process]));
            }
        }
    }

    return process_map;
}

fn is_found(args: Args, process_map: HashMap<String, Vec<Process>>) -> bool {
    for (tty, process_list) in process_map {
        if is_matched(args.clone(), tty, process_list) {
            return true;
        }
    }
    return false;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn make_target_is_tty() {
        let args = Args {
            command: None,
            pid: None,
            tty: Some(String::from("ttys000")),
        };
        let res = make_target(args);
        assert_eq!("tty: ttys000 ", res);
    }
    #[test]
    fn make_target_is_pid() {
        let args = Args {
            command: None,
            pid: Some(String::from("00000")),
            tty: Some(String::from("ttys000")),
        };
        let res = make_target(args);
        assert_eq!("pid: 00000 tty: ttys000 ", res);
    }

    #[test]
    fn make_target_is_command() {
        let args = Args {
            command: Some(String::from("command")),
            pid: Some(String::from("00000")),
            tty: Some(String::from("ttys000")),
        };
        let res = make_target(args);
        assert_eq!("command: command pid: 00000 tty: ttys000 ", res);
    }

    #[test]
    fn make_process_ok() {
        let process = "00000 ttys000    0:00.00 sleep 30";
        let res = make_process(process);
        assert_eq!("00000", res.1.pid);
        assert_eq!("ttys000", res.0);
        assert_eq!("sleep 30", res.1.command);
    }

    #[test]
    fn test_is_matched_true() {
        let args = Args {
            command: Some(String::from("command")),
            pid: None,
            tty: None,
        };
        let target_process = Process {
            pid: String::from("00000"),
            command: String::from("command"),
        };
        let is_continue = is_matched(args, String::from("ttys001"), Vec::from([target_process]));
        assert!(is_continue);
    }

    #[test]
    fn test_is_matched_false() {
        let args = Args {
            command: Some(String::from("ps")),
            pid: None,
            tty: None,
        };
        let target_process = Process {
            pid: String::from("00000"),
            command: String::from("command"),
        };
        let is_continue = is_matched(args, String::from("ttys001"), Vec::from([target_process]));
        assert!(!is_continue);
    }

    #[test]
    fn test_is_found_true() {
        let args = Args {
            command: Some(String::from("command")),
            pid: None,
            tty: None,
        };
        let process = Process {
            pid: String::from("00000"),
            command: String::from("command"),
        };
        let tty = String::from("ttys001");
        let process_list = Vec::from([process]);
        let mut process_map = HashMap::new();
        process_map.insert(tty, process_list);
        assert!(is_found(args, process_map))
    }

    #[test]
    fn test_is_found_false() {
        let args = Args {
            command: Some(String::from("ps")),
            pid: None,
            tty: None,
        };
        let process = Process {
            pid: String::from("00000"),
            command: String::from("command"),
        };
        let tty = String::from("ttys001");
        let process_list = Vec::from([process]);
        let mut process_map = HashMap::new();
        process_map.insert(tty, process_list);
        assert!(!is_found(args, process_map))
    }
}
