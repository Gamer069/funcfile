use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use sysinfo::{Disk, DiskKind, Disks};

#[derive(Clone, Debug)]
pub(crate) struct Volume {
    pub(crate) disk_type: DiskKind,
    pub(crate) name: String,
    pub(crate) mount_point: PathBuf,
    pub(crate) gb_left: f32,
    pub(crate) gb_used: f32,
    pub(crate) gb_total: f32,
}

impl Volume {
    pub fn new(disk_type: DiskKind, name: String, mount_point: PathBuf, gb_left: f32, gb_used: f32) -> Self {
        Self { disk_type, name, mount_point, gb_left, gb_used, gb_total: gb_used + gb_left }
    }
    pub fn from(disk: &Disk) -> Self {
        let mount_point = disk.mount_point().to_path_buf();
        let name = disk.name().to_str().unwrap().to_string();
        let gb_left = disk.available_space() as f32 / 1_073_741_824f32;
        let gb_total = disk.total_space() as f32 / 1_073_741_824f32;
        let gb_used = gb_total - gb_left;
        let disk_type = disk.kind();
        Self { disk_type, name, mount_point, gb_left, gb_total, gb_used }
    }
}