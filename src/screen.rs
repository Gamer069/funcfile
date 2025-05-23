use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use sysinfo::Disks;
use crate::fs::Volume;

#[derive(Clone)]
pub(crate) enum Screen {
    DriveSel(Vec<Volume>, Arc<Mutex<Disks>>),
    FileBrowse(Volume, PathBuf),
}