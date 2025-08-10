use std::io::{self, Write};

use image::DynamicImage;
use rasteroid::image_extended::InlineImage as _;
use rasteroid::term_misc::EnvIdentifiers;
use rasteroid::{InlineEncoder, inline_an_image};

pub trait ImageBackend {
    fn render_inline(&mut self, png_bytes: &[u8], left_offset_cells: u16) -> io::Result<()>;
    fn resize_for_width(&self, img: &DynamicImage, available_cells: usize) -> io::Result<Vec<u8>>;
}

pub struct RasteroidBackend;

impl Default for RasteroidBackend {
    fn default() -> Self {
        Self
    }
}

impl ImageBackend for RasteroidBackend {
    fn render_inline(&mut self, png_bytes: &[u8], left_offset_cells: u16) -> io::Result<()> {
        let mut env = EnvIdentifiers::new();
        let encoder = InlineEncoder::auto_detect(false, false, false, false, &mut env);
        let mut out = io::stdout();
        inline_an_image(png_bytes, &mut out, Some(left_offset_cells), None, &encoder)
            .map_err(|e| io::Error::other(e.to_string()))?;
        out.flush()
    }

    fn resize_for_width(&self, img: &DynamicImage, available_cells: usize) -> io::Result<Vec<u8>> {
        let dim = format!("{available_cells}c");
        let (resized_png, _offset, _w, _h) = img
            .resize_plus(Some(&dim), None, false, false)
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(resized_png)
    }
}
