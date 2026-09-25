//! Images mods ship: PNGs under `ui/img/`, named `mod:stem`. A `stem@2x.png`
//! beside `stem.png` is the same picture at twice the pixels, picked on
//! high-density displays. Pixels live here; the atlas places them on demand
//! (see `Text::image_slot`), so a full atlas can start over and re-place
//! them like glyphs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One decoded PNG.
pub struct ImageData {
    pub w: u32,
    pub h: u32,
    /// Pixels per logical pixel: 1 for `stem.png`, 2 for `stem@2x.png`.
    pub factor: u32,
    /// RGBA8, straight alpha.
    pub rgba: Vec<u8>,
    pub path: PathBuf,
}

#[derive(Default)]
pub struct Images {
    /// By name, each variant that shipped (1x, 2x).
    by_name: HashMap<String, Vec<ImageData>>,
    pub warnings: Vec<String>,
}

impl Images {
    /// Load every mod's `ui/img/*.png`. A file that fails to decode is a
    /// warning naming it, and the name is left missing.
    pub fn load(mods: &[(String, &Path)]) -> Images {
        let mut out = Images::default();
        for (id, dir) in mods {
            let Ok(rd) = std::fs::read_dir(dir.join("ui").join("img")) else { continue };
            let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
            files.sort();
            for path in files.into_iter().filter(|p| p.extension().is_some_and(|e| e == "png")) {
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
                let (stem, factor) = match stem.strip_suffix("@2x") {
                    Some(s) => (s, 2),
                    None => (stem, 1),
                };
                match decode(&path) {
                    Ok((w, h, rgba)) => {
                        let name = format!("{id}:{stem}");
                        let variants = out.by_name.entry(name).or_default();
                        variants.retain(|v| v.factor != factor);
                        variants.push(ImageData { w, h, factor, rgba, path: path.clone() });
                    }
                    Err(e) => {
                        out.warnings.push(format!("{id}/ui/img/{}: {e}", path.file_name().unwrap().to_string_lossy()))
                    }
                }
            }
        }
        out
    }

    /// The variant for a display scale: 2x from 1.5 up when it shipped,
    /// otherwise whatever did.
    pub fn pick(&self, name: &str, scale: f32) -> Option<&ImageData> {
        let v = self.by_name.get(name)?;
        let want = if scale >= 1.5 { 2 } else { 1 };
        v.iter().find(|d| d.factor == want).or_else(|| v.first())
    }

    pub fn names(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.by_name.keys().map(|s| s.as_str()).collect();
        v.sort();
        v
    }
}

/// Decode a PNG to RGBA8, whatever its colour type or depth.
pub fn decode(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (info.width, info.height);
    let n = (w * h) as usize;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..n * 4].to_vec(),
        png::ColorType::Rgb => buf[..n * 3].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf[..n * 2].chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => buf[..n].iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("indexed PNG left unexpanded".into()),
    };
    if w == 0 || h == 0 {
        return Err("empty image".into());
    }
    Ok((w, h, rgba))
}
