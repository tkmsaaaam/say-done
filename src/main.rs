use std::env;
use std::process::Command;
use std::{thread, time};

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("monitoring {}", args[1]);
    for _i in 0..8640 {
        let mut i = 0;
        let output = Command::new("ps").output().expect("failed");
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains(&args[1]) {
                i += 1;
            }
        }
        if i < 2 {
            Command::new("say").arg("Done!").output().expect("failed");
            break;
        } else {
            thread::sleep(time::Duration::from_millis(10000));
        }
    }
}
