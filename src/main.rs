#![feature(decl_macro)]

mod clip;
mod fs;
mod screen;

use crate::fs::Volume;
use crate::screen::Screen;
use eframe::egui::{self, Button, TextEdit};
use eframe::egui::{Id, PointerButton, PopupCloseBehavior, Ui, Window};
use eframe::epaint::{TextureHandle, Vec2};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use sysinfo::Disks;

fn main() {
    let native_opts = eframe::NativeOptions::default();
    eframe::run_native(
        "FuncFile",
        native_opts,
        Box::new(|cc| Ok(Box::new(FuncFile::new(cc)))),
    )
    .expect("Failed to run application");
}

struct FuncFile {
    screen: Screen,
    failed_to_delete: bool,
    failed_to_open: bool,
    failed_to_read_copied_file: bool,
    dir_tex: Option<TextureHandle>,
    file_tex: Option<TextureHandle>,
    paste_tex: Option<TextureHandle>,
    back_tex: Option<TextureHandle>,
    drive_sel_tex: Option<TextureHandle>,
}

impl FuncFile {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let disks = Disks::new_with_refreshed_list();
        let mut volumes = vec![];
        for disk in &disks {
            volumes.push(Volume::from(disk));
        }
        Self {
            screen: Screen::DriveSel(volumes, Arc::new(Mutex::new(disks))),
            failed_to_delete: false,
            failed_to_open: false,
            failed_to_read_copied_file: false,
            dir_tex: None,
            file_tex: None,
            paste_tex: None,
            back_tex: None,
            drive_sel_tex: None,
        }
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
            }
            _ => return,
        }
    }
    fn drive_sel(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let volumes = match self.screen.clone() {
            Screen::DriveSel(vols, _disks) => vols,
            _ => return,
        };
        for vol in volumes {
            let drive_group = ui.group(|ui| {
                if vol.name.is_empty() {
                    ui.heading("Root");
                } else {
                    ui.heading(format!("\"{}\"", vol.name)); // Heading with volume name
                }

                ui.horizontal(|ui| {
                    ui.label("Type: ");
                    ui.monospace(vol.disk_type.to_string());
                });

                ui.horizontal(|ui| {
                    ui.label("Mountpoint: ");
                    ui.monospace(vol.mount_point.to_str().unwrap_or("Invalid Path")); // Display mount point
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
                if drive_group.response.hovered()
                    && ctx.input(|i| i.pointer.button_clicked(PointerButton::Primary))
                {
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

        if self.failed_to_open {
            Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Failed to open file/directory. Perhaps you don't have permission?");
                    if ui.button("OK").clicked() {
                        self.failed_to_open = false;
                    }
                });
        }

        if self.failed_to_read_copied_file {
            Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Failed to paste file/directory. Failed to read file.");
                    if ui.button("OK").clicked() {
                        self.failed_to_read_copied_file = false;
                    }
                });
        }

        let mut back = false;
        ui.horizontal(|ui| {
            if let Screen::FileBrowse(_, ref mut cur) = self.screen {
                if cur.parent().is_some() {
                    if ui.add(Button::image(&self.back_tex.clone().unwrap())).clicked() {
                        *cur = cur.parent().unwrap().to_path_buf();
                    }
                }
                if ui.add(Button::image(&self.paste_tex.clone().unwrap())).clicked() {
                    let copied_path = clip::paste();
                    let copied_path = copied_path.trim().to_string();
                    println!("{}", copied_path);
                    let exists = std::fs::exists(copied_path.clone());
                    if exists.is_err() || exists.is_ok_and(|x| {
                        println!("{}", x);
                        !x
                    }) {
                        self.failed_to_read_copied_file = true;
                    } else {
                        std::fs::copy(copied_path.clone(), cur.join(Path::new(&copied_path.clone()).file_name().unwrap())).expect("Failed to paste file");
                    }
                }

                if ui.add(Button::image(&self.drive_sel_tex.clone().unwrap())).clicked() {
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
        if back {
            return;
        }

        let (_, path) = if let Screen::FileBrowse(vol, path) = self.screen.clone() {
            (vol, path)
        } else {
            return;
        };
        let mut entries = std::fs::read_dir(path.clone());
        if entries.is_err() {
            self.failed_to_open = true;
        }
        while entries.is_err() {
            if let Screen::FileBrowse(_, ref mut cur) = self.screen {
                *cur = path.parent().unwrap().to_path_buf();
                entries = std::fs::read_dir(cur.clone());
            }
        }
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for (i, f) in entries.unwrap().enumerate() {
                    let id = Id::new(format!("ctx_menu{}", i));

                    if f.is_err() {
                        continue;
                    }
                    let path = f.unwrap().path();
                    let img = if path.is_dir() {
                        &self.dir_tex.clone().unwrap()
                    } else {
                        &self.file_tex.clone().unwrap()
                    };
                    let btn = ui.add(
                        egui::Button::image_and_text(img, format!("{}", path.display()))
                            .min_size(Vec2::new(ui.available_width(), 0.0)),
                    );

                    if btn.clicked() {
                        if path.is_dir() {
                            if let Screen::FileBrowse(_, ref mut cur) = self.screen {
                                *cur = path.clone();
                            }
                        } else {
                            open::that_detached(path.to_str().unwrap())
                                .expect("Failed to open file");
                        }
                    }

                    if btn.secondary_clicked() {
                        ui.memory_mut(|mem| {
                            mem.toggle_popup(id);
                        });
                    }

                    egui::popup::popup_below_widget(
                        ui,
                        id,
                        &btn,
                        PopupCloseBehavior::CloseOnClickOutside,
                        |ui| {
                            if ui.button("Delete").clicked() {
                                if path.is_file() {
                                    ui.memory_mut(|mem| {
                                        mem.close_popup();
                                    });
                                    if std::fs::remove_file(path.clone()).is_err() {
                                        self.failed_to_delete = true;
                                    }
                                } else {
                                    ui.memory_mut(|mem| {
                                        mem.close_popup();
                                    });
                                    if std::fs::remove_dir_all(path.clone()).is_err() {
                                        self.failed_to_delete = true;
                                    }
                                }
                            }
                            if ui.button("Copy").clicked() {
                                ui.memory_mut(|mem| {
                                    mem.close_popup();
                                });
                                clip::copy(path.to_str().unwrap().to_string());
                            }
                        },
                    );
                }
            });
    }
}

impl eframe::App for FuncFile {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_drive_sel();

        screen::load_image!(self, "dir.png", "dir_image", dir_tex, ctx);
        screen::load_image!(self, "file.png", "file_image", file_tex, ctx);
        screen::load_image!(self, "paste.png", "paste_image", paste_tex, ctx);
        screen::load_image!(self, "back.png", "back_image", back_tex, ctx);
        screen::load_image!(self, "drive_sel.png", "drive_sel_image", drive_sel_tex, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.drive_sel(ctx, ui);
            self.file_browse(ctx, ui);
        });
    }
}
