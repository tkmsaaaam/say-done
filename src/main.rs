use clap::Parser;
use std::collections::HashMap;
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
    #[arg(short = 'o', long = "output")]
    output: Option<bool>,
    #[arg(short = 'i', long = "interval")]
    interval: Option<u8>,
}

struct Query {
    command: Option<String>,
    pid: Option<String>,
    tty: Option<String>,
}

struct Process {
    pid: String,
    command: String,
}

impl Args {
    fn make_query(self) -> Query {
        return Query::new(self.command, self.pid, self.tty);
    }

    fn is_some(&self) -> bool {
        return self.command.is_some() || self.pid.is_some() || self.tty.is_some();
    }

    fn is_output(&self) -> bool {
        const DEFAULT_OUTPUT: bool = true;
        return match self.output {
            Some(o) => o,
            None => DEFAULT_OUTPUT,
        };
    }

    fn get_interval(&self) -> u8 {
        const DEFAULT_INTERVAL: u8 = 10;
        return match self.interval {
            Some(i) => i,
            None => DEFAULT_INTERVAL,
        };
    }
}

impl Query {
    fn new(command: Option<String>, pid: Option<String>, tty: Option<String>) -> Query {
        return Query {
            command,
            pid,
            tty,
        };
    }

    fn make_str(&self) -> String {
        let mut name = String::from("(");

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
        name = name.trim_end().parse().unwrap();
        name = name + ")";
        return name;
    }

    fn is_found(&self, process_map: HashMap<String, Vec<Process>>) -> bool {
        for (tty, process_list) in process_map {
            if self.is_matched(tty, process_list) {
                return true;
            }
        }
        return false;
    }

    fn is_matched(&self, target_tty: String, target_process_list: Vec<Process>) -> bool {
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

impl Process {
    fn new(pid: String, command: String) -> Process {
        return Process {
            pid,
            command,
        };
    }
}

const PS_COMMAND_FAILED_MESSAGE: &str = "ps was failed.";
const ONE_MINUTE: u8 = 60;

fn main() {
    let query = match make_query() {
        Some(q) => q,
        None => std::process::exit(0),
    };
    let query_str = query.make_str();
    let is_output = Args::parse().is_output();
    println!("monitoring {}", query_str);
    const MAX_MONITORING_TIME: u32 = ONE_MINUTE as u32 * 60_u32 * 24_u32;
    let interval = Args::parse().get_interval();

    for i in 0..MAX_MONITORING_TIME / interval as u32 {
        let output = Command::new("ps").output().expect(PS_COMMAND_FAILED_MESSAGE);
        let process_map = make_process_map(output.clone());
        let is_continue = query.is_found(process_map);
        if is_continue {
            thread::sleep(time::Duration::from_secs(interval as u64));
            if is_every_minute(i, interval) && is_output {
                println!("{} minutes", i / 6);
            }
            continue;
        }
        if i == 0 {
            print_target_not_found(query_str, output);
            std::process::exit(0);
        }
        notify_terminate(query_str, i, interval);
        std::process::exit(0);
    }
    println!("{} has been running over an hour.", query_str);
}

fn make_query() -> Option<Query> {
    let args = Args::parse();

    if args.is_some() {
        let query = args.make_query();
        return Some(query);
    }
    println!("args is not present.");
    println!(
        "Process: \n{}",
        String::from_utf8_lossy(
            &Command::new("ps")
                .output()
                .expect(PS_COMMAND_FAILED_MESSAGE)
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

    return if command.trim_end().is_empty() && pid.trim_end().is_empty() && tty.trim_end().is_empty() {
        None
    } else {
        Some(Query::new(
            Some(String::from(command.trim_end())),
            Some(String::from(pid.trim_end())),
            Some(String::from(tty.trim_end())),
        ))
    };
}

fn is_every_minute(i: u32, interval: u8) -> bool {
    return i % (ONE_MINUTE / interval) as u32 == 0;
}

fn make_process(process: &str) -> (String, Process) {
    let process_split: Vec<&str> = process.split_whitespace().collect();
    let pid_index = 0;
    let tty_index = 1;
    let command_start_index = 3;
    let mut command = String::from(process_split[command_start_index]);
    if process_split.len() > (command_start_index + 1) {
        for i in (command_start_index + 1)..process_split.len() {
            command = command + " " + process_split[i]
        }
    }

    return (
        String::from(process_split[tty_index]),
        Process::new(String::from(process_split[pid_index]), command)
    );
}

fn make_process_map(output: Output) -> HashMap<String, Vec<Process>> {
    let self_pid = std::process::id();
    let mut process_map: HashMap<String, Vec<Process>> = HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.starts_with("  PID")
            || line.starts_with(&self_pid.to_string())
            || line.starts_with(PS_COMMAND_FAILED_MESSAGE)
        {
            continue;
        }
        let (tty, process) = make_process(line);
        process_map.entry(tty).or_insert(Vec::new()).push(process);
    }

    return process_map;
}

fn print_target_not_found(target: String, output: Output) {
    println!(
        "{} is not found. or {} is not started.\nps result:",
        target, target
    );
    println!("{:?}", String::from_utf8(output.stdout));
}

fn notify_terminate(target: String, i: u32, interval: u8) {
    Command::new("say")
        .arg("Done!")
        .output()
        .expect("say was failed.");
    println!("{} was finished. time: {}s", target, i * interval as u32);
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

    impl Args {
        fn new(command: Option<String>, pid: Option<String>, tty: Option<String>, output: Option<bool>, interval: Option<u8>) -> Args {
            return Args {
                command,
                pid,
                tty,
                output,
                interval,
            };
        }
    }

    #[test]
    fn make_query() {
        let args = Args::new(Some(String::from("command")), Some(String::from("pid")), Some(String::from("tty")), None, None);
        let query = args.make_query();
        assert_eq!("command", query.command.unwrap());
        assert_eq!("pid", query.pid.unwrap());
        assert_eq!("tty", query.tty.unwrap());
    }

    #[test]
    fn is_some_true_command() {
        let args = Args::new(Some(String::from("command")), None, None, None, None);
        assert!(args.is_some());
    }

    #[test]
    fn is_some_true_pid() {
        let args = Args::new(None, Some(String::from("pid")), None, None, None);
        assert!(args.is_some());
    }

    #[test]
    fn is_some_true_tty() {
        let args = Args::new(None, None, Some(String::from("tty")), None, None);
        assert!(args.is_some());
    }

    #[test]
    fn is_some_false() {
        let args = Args::new(None, None, None, None, None);
        assert!(!args.is_some());
    }

    #[test]
    fn make_is_output_none() {
        let args = Args::new(None, None, None, None, None);
        assert!(args.is_output())
    }

    #[test]
    fn make_is_output_true() {
        let args = Args::new(None, None, None, Some(true), None);
        assert!(args.is_output())
    }

    #[test]
    fn make_is_output_false() {
        let args = Args::new(None, None, None, Some(false), None);
        assert!(!args.is_output())
    }

    #[test]
    fn get_interval_default() {
        let args = Args::new(None, None, None, None, None);
        let interval = args.get_interval();
        assert_eq!(10_u8, interval);
    }

    #[test]
    fn get_interval_explicit() {
        let args = Args::new(None, None, None, None, Some(1_u8));
        let interval = args.get_interval();
        assert_eq!(1_u8, interval);
    }

    #[test]
    fn query_new() {
        let query = Query::new(Some(String::from("command")), Some(String::from("pid")), Some(String::from("tty")));
        assert_eq!("command", query.command.unwrap());
        assert_eq!("pid", query.pid.unwrap());
        assert_eq!("tty", query.tty.unwrap());
    }


    #[test]
    fn make_str_from_tty() {
        let query = Query::new(None, None, Some(String::from("ttys000")));
        let res = query.make_str();
        assert_eq!("(tty: ttys000)", res);
    }

    #[test]
    fn make_str_from_pid() {
        let query = Query::new(None, Some(String::from("00000")), Some(String::from("ttys000")));
        let res = query.make_str();
        assert_eq!("(pid: 00000 tty: ttys000)", res);
    }

    #[test]
    fn make_str_from_command() {
        let query = Query::new(Some(String::from("command")), Some(String::from("00000")), Some(String::from("ttys000")));
        let res = query.make_str();
        assert_eq!("(command: command pid: 00000 tty: ttys000)", res);
    }


    #[test]
    fn is_found_true() {
        let query = Query::new(Some(String::from("command")), None, None);
        let process = Process::new(String::from("00000"), String::from("command"));
        let tty = String::from("ttys001");
        let process_list = Vec::from([process]);
        let process_map = HashMap::from([(tty, process_list)]);
        assert!(query.is_found(process_map))
    }

    #[test]
    fn is_found_false() {
        let query = Query::new(Some(String::from("ps")), None, None);
        let process = Process::new(String::from("00000"), String::from("command"));
        let tty = String::from("ttys001");
        let process_list = Vec::from([process]);
        let process_map = HashMap::from([(tty, process_list)]);
        assert!(!query.is_found(process_map))
    }

    #[test]
    fn is_matched_true() {
        let query = Query::new(Some(String::from("command")), None, None);
        let process = Process::new(String::from("00000"), String::from("command"));
        let is_continue = query.is_matched(String::from("ttys001"), Vec::from([process]));
        assert!(is_continue);
    }

    #[test]
    fn is_matched_false() {
        let query = Query::new(Some(String::from("ps")), None, None);
        let process = Process::new(String::from("00000"), String::from("command"));
        let is_continue = query.is_matched(String::from("ttys001"), Vec::from([process]));
        assert!(!is_continue);
    }

    #[test]
    fn process_new() {
        let process = Process::new(String::from("pid"), String::from("command"));
        assert_eq!("pid", process.pid);
        assert_eq!("command", process.command);
    }

    #[test]
    fn make_process_ok() {
        let process = "00000 ttys000    0:00.00 sleep 30";
        let res = make_process(process);
        assert_eq!("00000", res.1.pid);
        assert_eq!("ttys000", res.0);
        assert_eq!("sleep 30", res.1.command);
    }
}
