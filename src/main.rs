use std::collections::BTreeMap;
use std::env::{self};
use std::process::{Command, Output};
use std::{thread, time};

use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    command: Option<String>,
    #[arg(short, long)]
    pid: Option<String>,
    #[arg(short, long)]
    tty: Option<String>,
    #[arg(short, long)]
    output: Option<bool>,
    #[arg(short, long)]
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
        return Query { command, pid, tty };
    }

    fn make_str(&self) -> String {
        let c = match self.command {
            Some(ref command) => String::from("command: ") + command + " ",
            None => String::new(),
        };

        let p = match self.pid {
            Some(ref pid) => String::from("pid: ") + pid + " ",
            None => String::new(),
        };

        let t = match self.tty {
            Some(ref tty) => String::from("tty: ") + tty + " ",
            None => String::new(),
        };
        return format!("({}{}{})", c, p, t);
    }

    fn is_found(&self, process_map: BTreeMap<String, Vec<Process>>) -> bool {
        return process_map
            .iter()
            .any(|(tty, process_list)| self.is_matched(tty, process_list));
    }

    fn is_matched(&self, target_tty: &str, target_process_list: &Vec<Process>) -> bool {
        match self.pid {
            Some(ref pid)
                if target_process_list
                    .iter()
                    .any(|process| process.pid.eq(pid)) =>
            {
                return true;
            }
            _ => (),
        }

        match self.tty {
            Some(ref tty) if target_tty.eq(tty) && target_process_list.len() > 1 => {
                return true;
            }
            _ => (),
        }

        match self.command {
            Some(ref command)
                if target_process_list
                    .iter()
                    .any(|process| process.command.starts_with(command)) =>
            {
                return true;
            }
            _ => (),
        }
        return false;
    }
}

impl Process {
    fn new(pid: String, command: String) -> Process {
        return Process { pid, command };
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
    println!("monitoring {}", &query_str);
    const MAX_MONITORING_TIME: u32 = ONE_MINUTE as u32 * 60u32 * 24u32;
    let interval = Args::parse().get_interval();

    for i in 0..MAX_MONITORING_TIME / interval as u32 {
        let ps_result = Command::new("ps")
            .output()
            .expect(PS_COMMAND_FAILED_MESSAGE);
        let process_map = make_process_map(&ps_result);
        if !query.is_found(process_map) {
            if i == 0 {
                print_target_not_found(&query_str, ps_result);
            } else {
                notify_terminate(&query_str, i, interval);
            }
            std::process::exit(0);
        }
        if is_every_minute(i, interval) && is_output {
            println!("{} minutes", elapsed_minute(i, interval));
        }
        thread::sleep(time::Duration::from_secs(interval as u64));
    }
    println!("{} has been running over an hour.", query_str);
}

fn make_query_element(element_name: &str) -> Option<String> {
    println!("{}:", element_name);
    let mut element = String::new();
    std::io::stdin().read_line(&mut element).expect("");
    if element.trim_end().is_empty() {
        return None;
    } else {
        Some(String::from(element.trim_end()))
    }
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

    let command = make_query_element("command");
    let pid = make_query_element("pid");
    let tty = make_query_element("tty");

    return if command.is_none() && pid.is_none() && tty.is_none() {
        None
    } else {
        Some(Query::new(command, pid, tty))
    };
}

fn is_every_minute(i: u32, interval: u8) -> bool {
    return i % (ONE_MINUTE / interval) as u32 == 0;
}

fn elapsed_minute(i: u32, interval: u8) -> u32 {
    return i / (ONE_MINUTE / interval) as u32;
}

fn make_process(process: &str) -> (String, Process) {
    let process_split: Vec<&str> = process.split_whitespace().collect();
    let pid_index = 0;
    let tty_index = 1;
    let command_start_index = 3;
    let command = process_split[command_start_index..process_split.len()].join(" ");

    return (
        String::from(process_split[tty_index]),
        Process::new(String::from(process_split[pid_index]), command),
    );
}

fn make_process_map(output: &Output) -> BTreeMap<String, Vec<Process>> {
    let self_pid = std::process::id();
    return String::from_utf8_lossy(&*output.stdout)
        .lines()
        .filter(|line| {
            !line.starts_with("  PID")
                && !line.starts_with(&self_pid.to_string())
                && !line.starts_with(PS_COMMAND_FAILED_MESSAGE)
        })
        .fold(
            BTreeMap::new(),
            |mut map: BTreeMap<String, Vec<Process>>, line| {
                let (tty, process) = make_process(line);
                map.entry(tty).or_insert_with(Vec::new).push(process);
                map
            },
        );
}

fn print_target_not_found(target: &String, output: Output) {
    println!(
        "{} is not found. or {} is not started.\nps result:",
        target, target
    );
    println!("{:?}", String::from_utf8(output.stdout));
}

fn notify_terminate(target: &String, i: u32, interval: u8) {
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
        fn new(
            command: Option<String>,
            pid: Option<String>,
            tty: Option<String>,
            output: Option<bool>,
            interval: Option<u8>,
        ) -> Args {
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
        let args = Args::new(
            Some(String::from("command")),
            Some(String::from("pid")),
            Some(String::from("tty")),
            None,
            None,
        );
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
        assert_eq!(10u8, interval);
    }

    #[test]
    fn get_interval_explicit() {
        let args = Args::new(None, None, None, None, Some(1u8));
        let interval = args.get_interval();
        assert_eq!(1u8, interval);
    }

    #[test]
    fn query_new() {
        let query = Query::new(
            Some(String::from("command")),
            Some(String::from("pid")),
            Some(String::from("tty")),
        );
        assert_eq!("command", query.command.unwrap());
        assert_eq!("pid", query.pid.unwrap());
        assert_eq!("tty", query.tty.unwrap());
    }

    #[test]
    fn make_str_from_tty() {
        let query = Query::new(None, None, Some(String::from("ttys000")));
        let res = query.make_str();
        assert_eq!("(tty: ttys000 )", res);
    }

    #[test]
    fn make_str_from_pid() {
        let query = Query::new(
            None,
            Some(String::from("00000")),
            Some(String::from("ttys000")),
        );
        let res = query.make_str();
        assert_eq!("(pid: 00000 tty: ttys000 )", res);
    }

    #[test]
    fn make_str_from_command() {
        let query = Query::new(
            Some(String::from("command")),
            Some(String::from("00000")),
            Some(String::from("ttys000")),
        );
        let res = query.make_str();
        assert_eq!("(command: command pid: 00000 tty: ttys000 )", res);
    }

    #[test]
    fn is_found_true() {
        let query = Query::new(Some(String::from("command")), None, None);
        let process = Process::new(String::from("00000"), String::from("command"));
        let tty = String::from("ttys001");
        let process_list = Vec::from([process]);
        let process_map = BTreeMap::from([(tty, process_list)]);
        assert!(query.is_found(process_map))
    }

    #[test]
    fn is_found_false() {
        let query = Query::new(Some(String::from("ps")), None, None);
        let process = Process::new(String::from("00000"), String::from("command"));
        let tty = String::from("ttys001");
        let process_list = Vec::from([process]);
        let process_map = BTreeMap::from([(tty, process_list)]);
        assert!(!query.is_found(process_map))
    }

    #[test]
    fn is_matched_true() {
        let query = Query::new(Some(String::from("command")), None, None);
        let process = Process::new(String::from("00000"), String::from("command"));
        let is_continue = query.is_matched("ttys001", Vec::from([process]).as_ref());
        assert!(is_continue);
    }

    #[test]
    fn is_matched_false() {
        let query = Query::new(Some(String::from("ps")), None, None);
        let process = Process::new(String::from("00000"), String::from("command"));
        let is_continue = query.is_matched("ttys001", Vec::from([process]).as_ref());
        assert!(!is_continue);
    }

    #[test]
    fn process_new() {
        let process = Process::new(String::from("pid"), String::from("command"));
        assert_eq!("pid", process.pid);
        assert_eq!("command", process.command);
    }

    #[test]
    fn is_every_minute_true() {
        let i_six = 6u32;
        let interval_ten = 10u8;
        assert!(is_every_minute(i_six, interval_ten));

        let i_twelve = 12u32;
        assert!(is_every_minute(i_twelve, interval_ten));

        let i_two = 2u32;
        let interval_thirty = 30u8;
        assert!(is_every_minute(i_two, interval_thirty));

        let i_four = 4u32;
        assert!(is_every_minute(i_four, interval_thirty));
    }

    #[test]
    fn is_every_minute_false() {
        let i_one = 1u32;
        let interval_ten = 10u8;
        assert!(!is_every_minute(i_one, interval_ten));

        let i_eight = 8u32;
        assert!(!is_every_minute(i_eight, interval_ten));

        let interval_thirty = 30u8;
        assert!(!is_every_minute(i_one, interval_thirty));
    }

    #[test]
    fn elapsed_minute_() {
        assert_eq!(1u32, elapsed_minute(6u32, 10u8));
        assert_eq!(1u32, elapsed_minute(3u32, 20u8));
    }

    #[test]
    fn make_process_ok() {
        let process = "00000 ttys000    0:00.00 sleep 30";
        let (tty, process) = make_process(process);
        assert_eq!("00000", process.pid);
        assert_eq!("ttys000", tty);
        assert_eq!("sleep 30", process.command);
    }
}
