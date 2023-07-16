extern crate libc;
use std::env;
use std::process::Command;
use std::{thread, time};

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("monitoring {}", args[1]);
    const INTERVAL: u64 = 10;
    const MAX_MONITERING_TIME: u64 = 60 * 60 * 24;
    let self_pid = unsafe { libc::getpid() };

    for _i in 0..MAX_MONITERING_TIME / INTERVAL {
        let mut target_process_count = 0;
        let output = Command::new("ps").output().expect("failed");
        for process in String::from_utf8_lossy(&output.stdout).lines() {
            let process_splited: Vec<&str> = process.split_whitespace().collect();
            if process_splited[0].eq(&self_pid.to_string()) {
                continue;
            }
            for i in 3..process_splited.len() {
                if process_splited[i].contains(&args[1]) {
                    target_process_count += 1;
                    break;
                }
            }
        }
        if target_process_count < 1 {
            Command::new("say").arg("Done!").output().expect("failed");
            break;
        } else {
            thread::sleep(time::Duration::from_secs(INTERVAL));
        }
    }
}
