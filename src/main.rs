use std::env;
use std::process::Command;
use std::{thread, time};

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("monitoring {}", args[1]);
    for _i in 0..8640 {
        let mut target_process_count = 0;
        let output = Command::new("ps").output().expect("failed");
        for process in String::from_utf8_lossy(&output.stdout).lines() {
            let process_splited: Vec<&str> = process.split_whitespace().collect();
            for i in 3..process_splited.len() {
                if process_splited[i].contains(&args[1]) {
                    target_process_count += 1;
                }
            }
        }
        if target_process_count < 2 {
            Command::new("say").arg("Done!").output().expect("failed");
            break;
        } else {
            thread::sleep(time::Duration::from_millis(10000));
        }
    }
}
