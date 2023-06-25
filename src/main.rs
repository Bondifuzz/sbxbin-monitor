mod cgroups;
mod config;
mod fsinfo;
mod pskill;

use cgroups::CGroups;
use config::Config;
use fsinfo::FSInfo;

use nix::sys::signal::Signal;
use serde_json::json;
use signal_hook::flag::register;

use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use std::{thread, time::Duration};

#[derive(Debug)]
enum ExitReason {
    TmpfsFull,
    Terminated,
    InternalError,
}

fn exit(reason: ExitReason) -> ! {
    match reason {
        // ExitReason::Finished => std::process::exit(0),
        ExitReason::TmpfsFull => std::process::exit(138), // SIGUSR1
        ExitReason::Terminated => std::process::exit(130), // SIGTERM
        ExitReason::InternalError => std::process::exit(-1),
    }
}

fn get_config_path() -> String {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: monitor <config.json>");
        exit(ExitReason::InternalError);
    }

    let config_path = args[1].clone();
    println!("Using config file: {config_path}");
    config_path
}

fn get_config(path: &str) -> Config {
    config::load_json(path).unwrap_or_else(|e| {
        println!("Failed to load config. Reason - {e}");
        exit(ExitReason::InternalError);
    })
}

fn get_cgroups() -> CGroups {
    let cgroups = CGroups::new().unwrap_or_else(|e| {
        println!("Failed init cgroups. Reason - {e}");
        exit(ExitReason::InternalError);
    });

    let mem_used_mb = cgroups.get_mem_usage_mb().unwrap_or_else(|e| {
        println!("Failed to get container memory usage. Reason - {e}");
        exit(ExitReason::InternalError);
    });

    println!("CGroups version: {:?}", cgroups.get_version());
    println!("Container memory usage (MB): {}", mem_used_mb);
    cgroups
}
fn get_tmpfs(volume_path: &str) -> FSInfo {
    let fs_info = FSInfo::new(volume_path).unwrap_or_else(|e| {
        println!("Failed to get tmpfs information. Reason - {e}");
        exit(ExitReason::InternalError);
    });

    let fs_usage = fs_info.space_usage_mb().unwrap_or_else(|e| {
        println!("Failed to get tmpfs space usage. Reason - {e}");
        exit(ExitReason::InternalError);
    });

    println!("TmpFS location: {volume_path}");
    println!("Total space (MB): {}", fs_usage.total);
    println!("Used space (MB): {}", fs_usage.used);
    fs_info
}

fn create_symlink(path: &str, link: &str) -> io::Result<()> {
    fs::remove_file(link).ok();
    symlink(&path, &link)?;
    Ok(())
}

fn create_symlinks(config: &Config) {
    for symlink in &config.symlinks {
        match create_symlink(&symlink.path, &symlink.link) {
            Ok(_) => println!("Created symlink: '{}' -> '{}'", symlink.link, symlink.path),
            Err(e) => {
                println!(
                    "Failed to create symlink: '{}' -> '{}'. Reason - {}",
                    symlink.link,
                    symlink.path,
                    e.to_string()
                );
                exit(ExitReason::InternalError);
            }
        }
    }
}

fn main() {
    let config_path = get_config_path();
    let config = get_config(&config_path);
    let cgroups = get_cgroups();
    let tmpfs = get_tmpfs(&config.tmpfs_volume_path);
    create_symlinks(&config);

    let exit_reason: ExitReason;
    let term = Arc::new(AtomicBool::new(false));

    let signals = [
        signal_hook::consts::SIGINT,  // rustfmt::skip
        signal_hook::consts::SIGTERM, // rustfmt::skip
    ];

    for signal in signals {
        register(signal, Arc::clone(&term)).unwrap_or_else(|e| {
            println!("Failed to register signal handlers. Reason - {e}");
            exit(ExitReason::InternalError);
        });
    }

    println!("Start monitoring");

    loop {
        let tmpfs_usage = match tmpfs.space_usage_mb() {
            Ok(val) => val,
            Err(e) => {
                println!("Failed to get tmpfs space usage. Reason - {e}. Exitting...");
                exit_reason = ExitReason::InternalError;
                break;
            }
        };

        let mem_used_mb = match cgroups.get_mem_usage_mb() {
            Ok(val) => val,
            Err(e) => {
                println!("Failed to get container memory usage. Reason - {e}. Exitting...");
                exit_reason = ExitReason::InternalError;
                break;
            }
        };

        let metrics_file = config.metrics_file_path.as_str();
        let metrics = json!({"memory": mem_used_mb, "tmpfs": tmpfs_usage.used });

        match fs::write(metrics_file, metrics.to_string()) {
            Ok(_) => (),
            Err(e) => {
                println!("Failed to dump metrics. Reason - {e}. Exitting...");
                exit_reason = ExitReason::InternalError;
                break;
            }
        };

        if tmpfs_usage.free < config.tmpfs_min_space_left_mb {
            println!("TmpFS is full. Exitting...");
            exit_reason = ExitReason::TmpfsFull;
            break;
        }

        if term.load(Ordering::Relaxed) {
            println!("Caught SIGTERM. Exitting...");
            exit_reason = ExitReason::Terminated;
            break;
        }

        let sleep_ms = config.metrics_dump_interval_ms;
        thread::sleep(Duration::from_millis(sleep_ms));
    }

    let runner_path = config.runner_binary_path.as_str();
    println!("Stopping all processes with file path '{}'", runner_path);
    pskill::killall(runner_path, Signal::SIGTERM);

    println!("Delay before exit...");
    let sleep_ms = config.grace_period_seconds;
    thread::sleep(Duration::from_secs(sleep_ms));

    println!("Exit. Reason - {exit_reason:?}");
    exit(exit_reason);
}
