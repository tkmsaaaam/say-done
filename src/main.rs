use std::collections::BTreeMap;
use std::env::{self};
use std::io::{BufRead, Write};
use std::process::{Command, Output};
use std::{io, thread, time};

use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    command: Option<String>,
    #[arg(short, long)]
    pid: Option<u32>,
    #[arg(short, long)]
    tty: Option<String>,
    #[arg(short, long)]
    output: Option<bool>,
    #[arg(short, long)]
    interval: Option<u8>,
}

struct Query {
    command: Option<String>,
    pid: Option<u32>,
    tty: Option<String>,
}

struct Process {
    pid: u32,
    command: String,
}

impl Args {
    fn make_query(self) -> Query {
        Query::new(self.command, self.pid, self.tty)
    }

    fn is_some(&self) -> bool {
        self.command.is_some() || self.pid.is_some() || self.tty.is_some()
    }

    fn is_output(&self) -> bool {
        const DEFAULT_OUTPUT: bool = true;
        self.output.unwrap_or(DEFAULT_OUTPUT)
    }

    fn get_interval(&self) -> u8 {
        const DEFAULT_INTERVAL: u8 = 10;
        self.interval.unwrap_or(DEFAULT_INTERVAL)
    }
}

impl Query {
    fn new(command: Option<String>, pid: Option<u32>, tty: Option<String>) -> Query {
        if pid.is_some() && pid.unwrap() > 99999 {
            return Query {
                command,
                pid: None,
                tty,
            };
        }
        Query { command, pid, tty }
    }

    fn make_str(&self) -> String {
        let c = match self.command {
            Some(ref command) => String::from("command: ") + command + " ",
            None => String::new(),
        };

        let p = match self.pid {
            Some(ref pid) => String::from("pid: ") + &pid.to_string() + " ",
            None => String::new(),
        };

        let t = match self.tty {
            Some(ref tty) => String::from("tty: ") + tty + " ",
            None => String::new(),
        };
        format!("({}{}{})", c, p, t)
    }

    fn is_found(&self, process_map: BTreeMap<String, Vec<Process>>) -> bool {
        process_map
            .iter()
            .any(|(tty, process_list)| self.is_matched(tty, process_list))
    }

    fn is_matched(&self, target_tty: &str, target_process_list: &[Process]) -> bool {
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
            Some(ref tty) if target_tty.eq(tty) && !target_process_list.is_empty() => {
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
        false
    }
}

impl Process {
    fn new(pid: u32, command: String) -> Process {
        Process { pid, command }
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
    let start_index = 0;

    for i in start_index..MAX_MONITORING_TIME / interval as u32 {
        let ps_result = Command::new("ps")
            .output()
            .expect(PS_COMMAND_FAILED_MESSAGE);
        let process_map = make_process_map(&ps_result);
        if !query.is_found(process_map) {
            if i == start_index {
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                print_target_not_found(&mut stdout, &query_str, &ps_result);
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

fn make_query_element<R: BufRead, W: Write>(
    mut reader: R,
    w: &mut W,
    element_name: &str,
) -> Option<String> {
    writeln!(w, "{}:", element_name).expect("can not write element_name");
    let mut element = String::new();
    reader.read_line(&mut element).expect("");
    if element.trim_end().is_empty() {
        None
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
    println!("Args is not present. Choose from these processes.");
    let ps_result = Command::new("ps")
        .output()
        .expect(PS_COMMAND_FAILED_MESSAGE);
    make_process_map(&ps_result).iter().for_each(|tty| {
        tty.1.iter().for_each(|process| {
            println!(
                "tty: {}, pid: {}, command: {}",
                tty.0, process.pid, process.command
            );
        })
    });

    let command = make_query_element(io::stdin().lock(), &mut io::stdout().lock(), "command");
    let pid_str = make_query_element(io::stdin().lock(), &mut io::stdout().lock(), "pid");
    let tty = make_query_element(io::stdin().lock(), &mut io::stdout().lock(), "tty");

    if command.is_none() && pid_str.is_none() && tty.is_none() {
        None
    } else {
        let pid = pid_str.map(|p| p.parse().unwrap());
        Some(Query::new(command, pid, tty))
    }
}

fn is_every_minute(i: u32, interval: u8) -> bool {
    i % (ONE_MINUTE / interval) as u32 == 0
}

fn elapsed_minute(i: u32, interval: u8) -> u32 {
    i / (ONE_MINUTE / interval) as u32
}

fn make_process(process: &str) -> (String, Process) {
    let process_split: Vec<&str> = process.split_whitespace().collect();
    let pid_index = 0;
    let tty_index = 1;
    let command_start_index = 3;
    let command = process_split[command_start_index..process_split.len()].join(" ");

    (
        String::from(process_split[tty_index]),
        Process::new(process_split[pid_index].parse().unwrap(), command),
    )
}

fn make_process_map(output: &Output) -> BTreeMap<String, Vec<Process>> {
    let self_pid = std::process::id();
    let shells = ["-bash", "-zsh"];
    String::from_utf8_lossy(&output.stdout)
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
                if !shells.contains(&&*process.command) {
                    map.entry(tty).or_default().push(process);
                }
                map
            },
        )
}

fn print_target_not_found<W: Write>(w: &mut W, target: &str, output: &Output) {
    writeln!(
        w,
        "{} is not found. or {} is not started.\nps result:",
        target, target
    )
    .expect("can not writeln");
    writeln!(w, "{}", String::from_utf8_lossy(&output.stdout)).expect("can not writeln");
}

fn notify_terminate(target: &String, i: u32, interval: u8) {
    Command::new("say")
        .arg("Done!")
        .output()
        .expect("say was failed.");
    println!("{} was finished. time: {}s", target, i * interval as u32);
    if env::consts::OS == "macos" {
        let arg = String::from("display notification \"")
            + target
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
            pid: Option<u32>,
            tty: Option<String>,
            output: Option<bool>,
            interval: Option<u8>,
        ) -> Args {
            Args {
                command,
                pid,
                tty,
                output,
                interval,
            }
        }
    }

    #[test]
    fn make_query() {
        let args = Args::new(
            Some(String::from("command")),
            Some(11111),
            Some(String::from("tty")),
            None,
            None,
        );
        let query = args.make_query();
        assert_eq!("command", query.command.unwrap());
        assert_eq!(11111, query.pid.unwrap());
        assert_eq!("tty", query.tty.unwrap());
    }

    #[test]
    fn is_some_true_command() {
        let args = Args::new(Some(String::from("command")), None, None, None, None);
        assert!(args.is_some());
    }

    #[test]
    fn is_some_true_pid() {
        let args = Args::new(None, Some(00000), None, None, None);
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
            Some(11111),
            Some(String::from("tty")),
        );
        assert_eq!("command", query.command.unwrap());
        assert_eq!(11111, query.pid.unwrap());
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
        let query = Query::new(None, Some(11111), Some(String::from("ttys000")));
        let res = query.make_str();
        assert_eq!("(pid: 11111 tty: ttys000 )", res);
    }

    #[test]
    fn make_str_from_command() {
        let query = Query::new(
            Some(String::from("command")),
            Some(11111),
            Some(String::from("ttys000")),
        );
        let res = query.make_str();
        assert_eq!("(command: command pid: 11111 tty: ttys000 )", res);
    }

    #[test]
    fn is_found_true() {
        let query = Query::new(Some(String::from("command")), None, None);
        let process = Process::new(11111, String::from("command"));
        let tty = String::from("ttys001");
        let process_list = Vec::from([process]);
        let process_map = BTreeMap::from([(tty, process_list)]);
        assert!(query.is_found(process_map))
    }

    #[test]
    fn is_found_false() {
        let query = Query::new(Some(String::from("ps")), None, None);
        let process = Process::new(11111, String::from("command"));
        let tty = String::from("ttys001");
        let process_list = Vec::from([process]);
        let process_map = BTreeMap::from([(tty, process_list)]);
        assert!(!query.is_found(process_map))
    }

    #[test]
    fn is_matched_true() {
        let query = Query::new(Some(String::from("command")), None, None);
        let process = Process::new(11111, String::from("command"));
        let is_continue = query.is_matched("ttys001", Vec::from([process]).as_ref());
        assert!(is_continue);
    }

    #[test]
    fn is_matched_false() {
        let query = Query::new(Some(String::from("ps")), None, None);
        let process = Process::new(11111, String::from("command"));
        let is_continue = query.is_matched("ttys001", Vec::from([process]).as_ref());
        assert!(!is_continue);
    }

    #[test]
    fn process_new() {
        let process = Process::new(11111, String::from("command"));
        assert_eq!(11111, process.pid);
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
    fn make_query_element_is_empty() {
        let command = b"ps";
        let mut buf = vec![];
        make_query_element(&command[..], &mut buf, "element_name");
        let expected = "element_name:\n".as_bytes().to_owned();
        assert_eq!(expected, buf);
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
        let process = "11111 ttys000    0:00.00 sleep 30";
        let (tty, process) = make_process(process);
        assert_eq!(11111, process.pid);
        assert_eq!("ttys000", tty);
        assert_eq!("sleep 30", process.command);
    }

    #[test]
    fn make_process_map_ok() {
        let stdout = "  PID TTY TIME CMD\n11111 ttys000 0:00:00 -bash\n11112 ttys000 0:00:00 ps\n11113 ttys001 0:00:00 -bash".as_bytes().to_owned();
        let output = Output {
            status: Default::default(),
            stdout,
            stderr: vec![],
        };
        let map = make_process_map(&output);
        assert_eq!(1, map.len());
        assert_eq!(1, map.get("ttys000").unwrap().len());
        assert_eq!("ps", map.get("ttys000").unwrap().first().unwrap().command);
        assert_eq!(11112, map.get("ttys000").unwrap().first().unwrap().pid);
    }

    #[test]
    fn print_target_not_found_ok() {
        let mut buf = vec![];
        let stdout = "PID TTY TIME CMD\n00000 ttys000 0:00:00 -bash\n00001 ttys000 0:00:00 ps\n00002 ttys001 0:00:00 -bash".as_bytes().to_owned();
        let output = Output {
            status: Default::default(),
            stdout,
            stderr: vec![],
        };
        print_target_not_found(&mut buf, "echo", &output);
        let expected = "echo is not found. or echo is not started.\nps result:\nPID TTY TIME CMD\n00000 ttys000 0:00:00 -bash\n00001 ttys000 0:00:00 ps\n00002 ttys001 0:00:00 -bash\n".as_bytes().to_owned();

        assert_eq!(expected, buf);
    }
}
