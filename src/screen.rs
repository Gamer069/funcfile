use crate::fs::Volume;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use sysinfo::Disks;

#[derive(Clone)]
pub(crate) enum Screen {
    DriveSel(Vec<Volume>, Arc<Mutex<Disks>>),
    FileBrowse(Volume, PathBuf, String),
}

pub macro load_image($self:ident,$image_name:literal,$image_id:literal,$texture_handle_var_name:ident,$ctx:ident) {
    if $self.$texture_handle_var_name.is_none() {
        let dir_bytes = std::fs::read(
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join("hires")
                .join($image_name),
        )
        .expect(concat!("Failed to read bytes of image ", $image_name));
        let img = image::load_from_memory(&dir_bytes)
            .expect("Failed to load dir image")
            .resize(50, 50, image::imageops::FilterType::Nearest);
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.as_flat_samples();

        let color_image =
            eframe::egui::ColorImage::from_rgba_premultiplied(size, pixels.as_slice());

        $self.$texture_handle_var_name = Some($ctx.load_texture(
            $image_id,
            color_image,
            eframe::egui::TextureOptions::default(),
        ));
    }
}
