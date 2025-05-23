use crate::fs::Volume;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use sysinfo::Disks;

#[derive(Clone)]
pub(crate) enum Screen {
    DriveSel(Vec<Volume>, Arc<Mutex<Disks>>),
    FileBrowse(Volume, PathBuf),
}
