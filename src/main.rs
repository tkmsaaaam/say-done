use clap::Parser;
use std::collections::HashMap;
use std::env::{self};
use std::process::{Command, Output};
use std::{thread, time};

#[derive(Parser, Clone)]
struct Args {
    #[arg(short = 'c', long = "command")]
    command: Option<String>,
    #[arg(short = 'p', long = "pid")]
    pid: Option<String>,
    #[arg(short = 't', long = "tty")]
    tty: Option<String>,
    #[arg(short = 'o', long = "output")]
    output: Option<bool>,
}

#[derive(Clone)]
struct Query {
    command: Option<String>,
    pid: Option<String>,
    tty: Option<String>,
}

impl Args {
    fn make_query(self) -> Query {
        return Query {
            command: self.command,
            pid: self.pid,
            tty: self.tty,
        };
    }

    fn is_some(self) -> bool {
        return self.command.is_some() || self.pid.is_some() || self.tty.is_some();
    }

    fn is_output(self) -> bool {
        match self.output {
            Some(o) => {
                if o == false {
                    return !DEFAULT_OUTPUT;
                } else {
                    return DEFAULT_OUTPUT;
                }
            }
            None => return DEFAULT_OUTPUT,
        }
    }
}

impl Query {
    fn make_str(self) -> String {
        let mut name = String::new();

        match self.command {
            Some(ref command) => name = name + "command: " + command + " ",
            None => {}
        }

        match self.pid {
            Some(ref pid) => name = name + "pid: " + pid + " ",
            None => {}
        }

        match self.tty {
            Some(ref tty) => name = name + "tty: " + tty + " ",
            None => {}
        }
        return name;
    }

    fn is_found(self, process_map: HashMap<String, Vec<Process>>) -> bool {
        for (tty, process_list) in process_map {
            if self.clone().is_matched(tty, process_list) {
                return true;
            }
        }
        return false;
    }

    fn is_matched(self, target_tty: String, target_process_list: Vec<Process>) -> bool {
        match self.pid {
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

        match self.tty {
            Some(ref tty) => {
                if target_tty.eq(tty) && target_process_list.len() > 1 {
                    return true;
                }
            }
            None => {}
        }

        match self.command {
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
}

struct Process {
    pid: String,
    command: String,
}

const INTERVAL: u64 = 10;
const PS_COMMAND_FAILD_MESSAGE: &str = "ps was failed.";
const DEFAULT_OUTPUT: bool = true;

fn main() {
    let query = match make_query() {
        Some(q) => q,
        None => std::process::exit(0),
    };
    let query_str = query.clone().make_str();
    let is_output = Args::parse().is_output();
    println!("monitoring ({})", query_str);
    const MAX_MONITERING_TIME: u64 = 60 * 60 * 24;

    for i in 0..MAX_MONITERING_TIME / INTERVAL {
        let output = Command::new("ps").output().expect(PS_COMMAND_FAILD_MESSAGE);
        let process_map = make_process_map(output.clone());
        let is_continue = query.clone().is_found(process_map);
        if is_continue {
            thread::sleep(time::Duration::from_secs(INTERVAL));
            if i % 6 == 0 {
                if is_output {
                    println!("{} minutes", i / 6);
                }
            }
            continue;
        }
        if i == 0 {
            print_target_not_found(query_str, output);
            std::process::exit(0);
        }
        notify_terminate(query_str, i);
        std::process::exit(0);
    }
    println!("({}) has been running over an hour.", query_str);
}

fn make_query() -> Option<Query> {
    let args = Args::parse();

    if args.clone().is_some() {
        let query = args.make_query();
        return Some(query);
    }
    println!("args is not present.");
    println!(
        "Process: \n{}",
        String::from_utf8_lossy(
            &Command::new("ps")
                .output()
                .expect(PS_COMMAND_FAILD_MESSAGE)
                .stdout
        )
    );

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
        return Some(Query {
            command: Some(String::from(command.trim_end())),
            pid: Some(String::from(pid.trim_end())),
            tty: Some(String::from(tty.trim_end())),
        });
    }
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

fn make_process_map(output: Output) -> HashMap<String, Vec<Process>> {
    let self_pid = std::process::id();
    let mut process_map: HashMap<String, Vec<Process>> = HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.starts_with("  PID")
            || line.starts_with(&self_pid.to_string())
            || line.starts_with(PS_COMMAND_FAILD_MESSAGE)
        {
            continue;
        }
        let tty: String;
        let process: Process;
        (tty, process) = make_process(line);
        process_map.entry(tty).or_insert(Vec::new()).push(process);
    }

    return process_map;
}

fn print_target_not_found(target: String, output: Output) {
    println!(
        "({}) is not found. or ({}) is not started.\nps result:",
        target, target
    );
    println!("{:?}", String::from_utf8(output.stdout));
}

fn notify_terminate(target: String, i: u64) {
    Command::new("say")
        .arg("Done!")
        .output()
        .expect("say was failed.");
    println!("({}) was finished. time: {}s", target, i * INTERVAL);
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
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn make_query() {
        let args = Args {
            command: Some(String::from("command")),
            pid: Some(String::from("pid")),
            tty: Some(String::from("tty")),
            output: None,
        };
        let query = args.make_query();
        assert_eq!("command", query.command.unwrap());
        assert_eq!("pid", query.pid.unwrap());
        assert_eq!("tty", query.tty.unwrap());
    }

    #[test]
    fn make_query_str_from_tty() {
        let query = Query {
            command: None,
            pid: None,
            tty: Some(String::from("ttys000")),
        };
        let res = query.make_str();
        assert_eq!("tty: ttys000 ", res);
    }
    #[test]
    fn make_query_str_from_pid() {
        let query = Query {
            command: None,
            pid: Some(String::from("00000")),
            tty: Some(String::from("ttys000")),
        };
        let res = query.make_str();
        assert_eq!("pid: 00000 tty: ttys000 ", res);
    }

    #[test]
    fn make_query_str_from_command() {
        let query = Query {
            command: Some(String::from("command")),
            pid: Some(String::from("00000")),
            tty: Some(String::from("ttys000")),
        };
        let res = query.make_str();
        assert_eq!("command: command pid: 00000 tty: ttys000 ", res);
    }

    #[test]
    fn make_is_output_none() {
        let args = Args {
            command: None,
            pid: None,
            tty: None,
            output: None,
        };
        assert!(args.is_output())
    }

    #[test]
    fn make_is_output_true() {
        let args = Args {
            command: None,
            pid: None,
            tty: None,
            output: Some(true),
        };
        assert!(args.is_output())
    }

    #[test]
    fn make_is_output_false() {
        let args = Args {
            command: None,
            pid: None,
            tty: None,
            output: Some(false),
        };
        assert!(!args.is_output())
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
    fn is_matched_true() {
        let query = Query {
            command: Some(String::from("command")),
            pid: None,
            tty: None,
        };
        let target_process = Process {
            pid: String::from("00000"),
            command: String::from("command"),
        };
        let is_continue = query.is_matched(String::from("ttys001"), Vec::from([target_process]));
        assert!(is_continue);
    }

    #[test]
    fn is_matched_false() {
        let query = Query {
            command: Some(String::from("ps")),
            pid: None,
            tty: None,
        };
        let target_process = Process {
            pid: String::from("00000"),
            command: String::from("command"),
        };
        let is_continue = query.is_matched(String::from("ttys001"), Vec::from([target_process]));
        assert!(!is_continue);
    }

    #[test]
    fn is_found_true() {
        let query = Query {
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
        assert!(query.is_found(process_map))
    }

    #[test]
    fn is_found_false() {
        let query = Query {
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
        assert!(!query.is_found(process_map))
    }
}
