use std::io::{self, Write};

use image::DynamicImage;
use rasteroid::image_extended::InlineImage as _;
use rasteroid::term_misc::EnvIdentifiers;
use rasteroid::{InlineEncoder, inline_an_image};

pub trait ImageBackend {
    fn render_inline(&mut self, png_bytes: &[u8], left_offset_cells: u16) -> io::Result<()>;
    /// Resize to fit available cell width; returns PNG bytes and the target cell width used
    fn resize_for_width(
        &self,
        img: &DynamicImage,
        available_cells: usize,
    ) -> io::Result<(Vec<u8>, usize)>;
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

    fn resize_for_width(
        &self,
        img: &DynamicImage,
        available_cells: usize,
    ) -> io::Result<(Vec<u8>, usize)> {
        let dim = format!("{available_cells}c");
        let (resized_png, _offset, w_px, _h_px) = img
            .resize_plus(Some(&dim), None, false, false)
            .map_err(|e| io::Error::other(e.to_string()))?;
        // Convert resulting pixel width to terminal cells for accurate caption centering
        let cols = rasteroid::term_misc::dim_to_cells(
            &format!("{w_px}px"),
            rasteroid::term_misc::SizeDirection::Width,
        )
        .unwrap_or(available_cells as u32) as usize;
        Ok((resized_png, cols))
    }
}
