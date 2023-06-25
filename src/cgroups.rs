use std::cmp;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum CGroupVersion {
    V1,
    V2,
}

#[derive(Debug)]
pub struct CGroups {
    cgroup_ver: CGroupVersion,
    mem_usage_file: String,
    mem_stat_file: String,
}

impl CGroups {
    fn v1() -> Self {
        Self {
            cgroup_ver: CGroupVersion::V1,
            mem_usage_file: String::from("/sys/fs/cgroup/memory/memory.usage_in_bytes"),
            mem_stat_file: String::from("/sys/fs/cgroup/memory/memory.stat"),
        }
    }

    fn v2() -> Self {
        Self {
            cgroup_ver: CGroupVersion::V2,
            mem_usage_file: String::from("/sys/fs/cgroup/memory.current"),
            mem_stat_file: String::from("/sys/fs/cgroup/memory.stat"),
        }
    }

    pub fn new() -> Result<Self, String> {
        let mon = CGroups::v1();
        if mon.cgroup_files_exist() {
            return Ok(mon);
        }

        let mon = CGroups::v2();
        if mon.cgroup_files_exist() {
            return Ok(mon);
        }

        Err(String::from("Failed to get cgroups version"))
    }

    fn cgroup_files_exist(&self) -> bool {
        Path::new(&self.mem_usage_file).exists() && Path::new(&self.mem_stat_file).exists()
    }

    #[rustfmt::skip]
    fn read_to_string(path: &str) -> Result<String, String> {
        match fs::read_to_string(path) {
            Ok(val) => Ok(val),
            Err(e) => Err(format!(
                "Failed to read file {}. Reason - {}",
                 path, e.to_string()
            )),
        }
    }

    #[rustfmt::skip]
    fn parse_integer(value: &str) -> Result<usize, String> {
        match value.parse::<usize>() {
            Ok(val) => Ok(val),
            Err(e) => Err(format!(
                "Failed to parse {}. Reason - {}",
                 value, e.to_string()
            )),
        }
    }

    fn parse_key_val(value: &str) -> Result<(&str, &str), String> {
        match value.split_once(" ") {
            Some(val) => Ok(val),
            None => Err(format!("Failed to parse {} as <key, val>", value)),
        }
    }

    fn get_container_memory_usage_in_bytes(&self) -> Result<usize, String> {
        let content = CGroups::read_to_string(self.mem_usage_file.as_str())?;
        let mem_usage = CGroups::parse_integer(content.trim())?;
        Ok(mem_usage)
    }

    fn get_container_memory_stats(&self) -> Result<HashMap<String, usize>, String> {
        let mut stats = HashMap::new();
        let content = CGroups::read_to_string(self.mem_stat_file.as_str())?;

        for line in content.trim().split("\n") {
            let (name, value) = CGroups::parse_key_val(line)?;
            stats.insert(String::from(name), CGroups::parse_integer(value)?);
        }

        Ok(stats)
    }

    fn get_container_memory_working_set_in_bytes(&self) -> Result<usize, String> {
        let memory_stats = self.get_container_memory_stats()?;
        let memory_usage = self.get_container_memory_usage_in_bytes()?;

        if let Some(value) = memory_stats.get("total_inactive_file") {
            return Ok(cmp::max(0, memory_usage - value));
        }

        if let Some(value) = memory_stats.get("inactive_file") {
            return Ok(cmp::max(0, memory_usage - value));
        }

        Err("Unreachable: failed to get inactive_file stat".to_string())
    }

    pub fn get_mem_usage_mb(&self) -> Result<usize, String> {
        Ok(self.get_container_memory_working_set_in_bytes()? >> 20)
    }

    pub fn get_version(&self) -> &CGroupVersion {
        &self.cgroup_ver
    }
}
