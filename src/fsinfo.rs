use std::path::Path;

use nix::errno::Errno;
use nix::sys::statvfs::statvfs;

#[derive(Debug)]
pub struct FSInfo {
    volume_path: String,
}

#[derive(Debug)]
pub struct FSUsageMB {
    pub used: u64,
    pub free: u64,
    pub total: u64,
}

impl FSInfo {
    pub fn new(volume_path: &str) -> Result<Self, String> {
        if !Path::new(volume_path).exists() {
            return Err(format!("No filesystem found on {volume_path}"));
        }

        match statvfs(volume_path) {
            Ok(_) => Ok(FSInfo {
                volume_path: String::from(volume_path),
            }),
            Err(e) => Err(FSInfo::fmt_syscall_failed(e)),
        }
    }

    fn fmt_syscall_failed(e: Errno) -> String {
        format!("Syscall 'statvfs' failed. Errno: {e}")
    }

    pub fn space_usage_mb(self: &Self) -> Result<FSUsageMB, String> {
        let st = match statvfs(self.volume_path.as_str()) {
            Err(e) => return Err(FSInfo::fmt_syscall_failed(e)),
            Ok(st) => st,
        };

        let total = (st.blocks() * st.block_size()) >> 20;
        let avail = (st.blocks_available() * st.block_size()) >> 20;

        Ok(FSUsageMB {
            total: total,
            used: total - avail,
            free: avail,
        })
    }
}
