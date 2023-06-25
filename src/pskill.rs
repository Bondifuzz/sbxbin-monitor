use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use procfs::process::all_processes;
use std::path::Path;

pub fn killall(ps_binary_path: &str, signal: Signal) {
    let ps_binary_abspath = match Path::new(ps_binary_path).canonicalize() {
        Ok(val) => val,
        Err(_) => {
            println!("Failed to get absolute path for {ps_binary_path}");
            return;
        }
    };

    let process_list = match all_processes() {
        Ok(val) => val,
        Err(e) => {
            println!("Failed to list processes. Reason -{e:?}");
            return;
        }
    };

    for ps_entry in process_list {
        let process = match ps_entry {
            Ok(val) => val,
            Err(e) => {
                println!("Failed to get process. Reason - {e:?}");
                continue;
            }
        };

        let pid = process.pid();
        let binary_abspath = match process.exe() {
            Ok(pb) => pb,
            Err(_) => {
                println!("Failed to get process exe <pid={pid}>");
                continue;
            }
        };

        if ps_binary_abspath == binary_abspath {
            println!("Send {} to process <pid={}>", signal.as_str(), pid);
            signal::kill(Pid::from_raw(pid), signal).ok();
        }
    }
}
