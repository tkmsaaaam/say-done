use std::env;
use std::process::Command;
use std::{thread, time};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("CMD is not defined.\nrun: cargo run CMD");
        std::process::exit(0);
    }
    println!("monitoring {}", args[1]);
    const INTERVAL: u64 = 10;
    const MAX_MONITERING_TIME: u64 = 60 * 60 * 24;
    let self_pid = std::process::id();

    for i in 0..MAX_MONITERING_TIME / INTERVAL {
        let mut target_process_is_existed = false;
        let output = Command::new("ps").output().expect("failed");
        for process in String::from_utf8_lossy(&output.stdout).lines() {
            let process_splited: Vec<&str> = process.split_whitespace().collect();
            if process_splited[0].eq("PID") || process_splited[0].eq(&self_pid.to_string()) {
                continue;
            }
            if process_splited[3].starts_with(&args[1]) {
                target_process_is_existed = true;
                break;
            }
        }
        if !target_process_is_existed && i == 0 {
            println!(
                "{} is not found. or {} is not started.\nps result:",
                args[1], args[1]
            );
            println!("{}", String::from_utf8_lossy(&output.stdout));
            std::process::exit(0);
        }
        if !target_process_is_existed {
            Command::new("say").arg("Done!").output().expect("failed");
            println!("time: {}s", i * INTERVAL);
            if env::consts::OS == "macos" {
                let arg = String::from("display notification \"")
                    + &args[1]
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
        thread::sleep(time::Duration::from_secs(INTERVAL));
    }
    println!("{} has been running over an hour.", &args[1]);
}
