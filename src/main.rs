mod fs;
mod screen;

use std::sync::{Arc, Mutex};
use eframe::egui;
use eframe::egui::{Id, Image, PointerButton, PopupCloseBehavior, TextureOptions, Ui, Window};
use eframe::epaint::{TextureHandle, TextureId, Vec2};
use image::imageops::FilterType;
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
    failed_to_open: bool,
    dir_tex: Option<TextureHandle>,
    file_tex: Option<TextureHandle>
}

impl FuncFile {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let disks = Disks::new_with_refreshed_list();
        let mut volumes = vec![];
        for disk in &disks {
            volumes.push(Volume::from(disk));
        }
        let out = Self { screen: Screen::DriveSel(volumes, Arc::new(Mutex::new(disks))), failed_to_delete: false, failed_to_open: false, dir_tex: None, file_tex: None };
        out
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
            Screen::DriveSel(vols, _disks) => vols,
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

        let (_, path) = if let Screen::FileBrowse(vol, path) = self.screen.clone() { (vol, path) } else { return };
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
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for (i, f) in entries.unwrap().enumerate() {
                let id = Id::new(format!("ctx_menu{}", i));

                if f.is_err() {
                    continue;
                }
                let path = f.unwrap().path();
                let img = if path.is_dir() { &self.dir_tex.clone().unwrap() } else { &self.file_tex.clone().unwrap() };
                let btn = ui.add(egui::Button::image_and_text(img, format!("{}", path.display())).min_size(Vec2::new(ui.available_width(), 0.0)));

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
        });
    }
}

impl eframe::App for FuncFile {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_drive_sel();
        if self.dir_tex.is_none() {
            let dir_bytes = std::fs::read(std::env::current_exe().unwrap().parent().unwrap().join("dir.png")).expect("Failed to read dir image bytes");
            let img = image::load_from_memory(&dir_bytes).expect("Failed to load dir image").resize(50, 50, FilterType::Nearest);
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.as_flat_samples();

            let color_image = egui::ColorImage::from_rgba_premultiplied(
                size,
                pixels.as_slice(),
            );

            self.dir_tex = Some(ctx.load_texture("dir_image", color_image, TextureOptions::default()));
        }
        if self.file_tex.is_none() {
            let file_bytes = std::fs::read(std::env::current_exe().unwrap().parent().unwrap().join("file.png")).expect("Failed to read file image bytes");
            let img = image::load_from_memory(&file_bytes).expect("Failed to load file image").resize(50, 50, FilterType::Nearest);
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.as_flat_samples();

            let color_image = egui::ColorImage::from_rgba_premultiplied(
                size,
                pixels.as_slice(),
            );

            self.file_tex = Some(ctx.load_texture("file_image", color_image, TextureOptions::default()));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.drive_sel(ctx, ui);
            self.file_browse(ctx, ui);
        });
    }
}