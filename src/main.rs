mod fs;
mod screen;

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use eframe::egui;
use eframe::egui::{Id, PointerButton, PopupCloseBehavior, Ui, Window};
use eframe::egui::debug_text::print;
use sysinfo::Disks;
use crate::fs::Volume;
use crate::screen::Screen;

fn main() {
    let native_opts = eframe::NativeOptions::default();
    eframe::run_native("FuncFile", native_opts, Box::new(|cc| Ok(Box::new(FuncFile::new(cc))))).expect("Failed to run application");
}

struct FuncFile {
    screen: Screen,
    failed_to_delete: bool,
}

impl FuncFile {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let disks = Disks::new_with_refreshed_list();
        let mut volumes = vec![];
        for disk in &disks {
            volumes.push(Volume::from(disk));
        }
        Self { screen: Screen::DriveSel(volumes, Arc::new(Mutex::new(disks))), failed_to_delete: false }
    }
    fn refresh_drive_sel(&mut self) {
        match self.screen {
            Screen::DriveSel(ref mut volumes, ref mut disks) => {
                {
                    let mut data = disks.lock().unwrap();
                    data.refresh(true);
                }
                volumes.clear();
                for disk in disks.lock().unwrap().iter() {
                    volumes.push(Volume::from(disk));
                }
            },
            _ => return,
        }
    }
    fn drive_sel(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let volumes = match self.screen.clone() {
            Screen::DriveSel(vols, disks) => vols,
            _ => return,
        };
        for vol in volumes {
            let drive_group = ui.group(|ui| {
                if vol.name.is_empty() {
                    ui.heading("Root");
                } else {
                    ui.heading(format!("\"{}\"", vol.name));  // Heading with volume name
                }

                ui.horizontal(|ui| {
                    ui.label("Type: ");
                    ui.monospace(vol.disk_type.to_string());
                });

                ui.horizontal(|ui| {
                    ui.label("Mountpoint: ");
                    ui.monospace(vol.mount_point.to_str().unwrap_or("Invalid Path"));  // Display mount point
                });

                ui.horizontal(|ui| {
                    ui.monospace(format!("{} GB", vol.gb_used.to_string()));
                    ui.label("/");
                    ui.monospace(format!("{} GB", vol.gb_total.to_string()));
                });

                ui.horizontal(|ui| {
                    ui.label("Space left: ");
                    ui.monospace(vol.gb_left.to_string());
                });
            });
            let mouse = ctx.pointer_latest_pos().is_some();
            if mouse {
                if drive_group.response.hovered() && ctx.input(|i| i.pointer.button_clicked(PointerButton::Primary)) {
                    self.screen = Screen::FileBrowse(vol.clone(), vol.mount_point);
                }
            }
        }
    }
    fn file_browse(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        if let Screen::DriveSel(..) = self.screen.clone() {
            return;
        }

        if self.failed_to_delete {
            Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Failed to delete file. Make sure you have permission");
                    if ui.button("OK").clicked() {
                        self.failed_to_delete = false;
                    }
                });
        }

        let mut back = false;
        ui.horizontal(|ui| {
            if let Screen::FileBrowse(_, ref mut cur) = self.screen {
                if cur.parent().is_some() {
                    if ui.button("..\\").clicked() {
                        *cur = cur.parent().unwrap().to_path_buf();
                    }
                }
                if ui.button("Back to drive sel").clicked() {
                    let disks = Disks::new_with_refreshed_list();
                    let mut volumes = vec![];
                    for disk in &disks {
                        volumes.push(Volume::from(disk));
                    }
                    self.screen = Screen::DriveSel(volumes, Arc::new(Mutex::new(disks)));
                    back = true;
                }
            }
        });
        if back { return; }

        let (volume, path) = if let Screen::FileBrowse(vol, path) = self.screen.clone() { (vol, path) } else { return };
        let entries = std::fs::read_dir(path.clone());
        while entries.is_err() {
            if let Screen::FileBrowse(_, ref mut cur) = self.screen {
                *cur = path.parent().unwrap().to_path_buf();
            }
        }
        for (i, f) in entries.unwrap().enumerate() {
            let id = Id::new(format!("ctx_menu{}", i));

            if f.is_err() {
                continue;
            }
            let path = f.unwrap().path();
            let btn = ui.button(format!("{}", path.display()));
            if btn.clicked() {
                if path.is_dir() {
                    if let Screen::FileBrowse(_, ref mut cur) = self.screen {
                        *cur = path.clone();
                    }
                } else {
                    open::that_detached(path.to_str().unwrap()).expect("Failed to open file");
                }
            }

            if btn.secondary_clicked() {
                ui.memory_mut(|mem| {
                    mem.toggle_popup(id);
                });
            }

            egui::popup::popup_below_widget(ui, id, &btn, PopupCloseBehavior::CloseOnClickOutside, |ui| {
                if ui.button("Delete").clicked() {
                    if path.is_file() {
                        ui.close_menu();
                        if std::fs::remove_file(path.clone()).is_err() {
                            self.failed_to_delete = true;
                        }
                    } else {
                        ui.close_menu();
                        if std::fs::remove_dir_all(path.clone()).is_err() {
                            self.failed_to_delete = true;
                        }
                    }
                }
            });
        }
    }
}

impl eframe::App for FuncFile {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.refresh_drive_sel();
        egui::CentralPanel::default().show(ctx, |ui| {
            self.drive_sel(ctx, ui);
            self.file_browse(ctx, ui);
        });
    }
}
