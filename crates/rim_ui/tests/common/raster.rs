//! A software rasteriser for the UI's draw list: enough to look at a
//! panel as a PNG without a window. Rects, hairlines, glyphs from the
//! atlas, and the clip; rounded corners are drawn square.
#![allow(dead_code)]

use rim_ui::paint::Draw;
use rim_ui::text::Atlas;

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[f32; 4]>,
    clip: Option<[f32; 4]>,
}

impl Canvas {
    pub fn new(w: usize, h: usize, bg: [f32; 4]) -> Canvas {
        Canvas { w, h, px: vec![bg; w * h], clip: None }
    }

    fn blend(&mut self, x: i32, y: i32, c: [f32; 4]) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h || c[3] <= 0.0 {
            return;
        }
        if let Some(cl) = self.clip {
            if (x as f32) < cl[0] || (y as f32) < cl[1] || x as f32 >= cl[0] + cl[2] || y as f32 >= cl[1] + cl[3] {
                return;
            }
        }
        let p = &mut self.px[y as usize * self.w + x as usize];
        let a = c[3].min(1.0);
        for i in 0..3 {
            p[i] = p[i] * (1.0 - a) + c[i] * a;
        }
        p[3] = 1.0;
    }

    fn fill(&mut self, r: [f32; 4], c: [f32; 4]) {
        let (x0, y0) = (r[0].round() as i32, r[1].round() as i32);
        let (x1, y1) = ((r[0] + r[2]).round() as i32, (r[1] + r[3]).round() as i32);
        for y in y0..y1 {
            for x in x0..x1 {
                self.blend(x, y, c);
            }
        }
    }

    pub fn draw(&mut self, list: &[Draw], atlas: &Atlas) {
        for d in list {
            match d {
                Draw::Rect { rect, color, .. } => self.fill(*rect, *color),
                Draw::Outline { rect, color, width, .. } => {
                    let w = width.max(1.0);
                    self.fill([rect[0], rect[1], rect[2], w], *color);
                    self.fill([rect[0], rect[1] + rect[3] - w, rect[2], w], *color);
                    self.fill([rect[0], rect[1], w, rect[3]], *color);
                    self.fill([rect[0] + rect[2] - w, rect[1], w, rect[3]], *color);
                }
                Draw::Glyphs { quads, color } => {
                    for q in quads {
                        let (dw, dh) = (q.dst[2].round() as i32, q.dst[3].round() as i32);
                        for j in 0..dh {
                            for i in 0..dw {
                                // Nearest sample: the quad is the glyph's own size unless it is an image.
                                let u = q.uv[0] + (i as f32 + 0.5) * q.uv[2] / dw.max(1) as f32;
                                let v = q.uv[1] + (j as f32 + 0.5) * q.uv[3] / dh.max(1) as f32;
                                let idx = ((v as u32) * atlas.size + u as u32) as usize * 4;
                                let Some(px) = atlas.pixels.get(idx..idx + 4) else { continue };
                                let a = px[3] as f32 / 255.0;
                                let c = if q.color {
                                    [px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0, a]
                                } else {
                                    [color[0], color[1], color[2], a * color[3]]
                                };
                                self.blend(q.dst[0].round() as i32 + i, q.dst[1].round() as i32 + j, c);
                            }
                        }
                    }
                }
                Draw::Clip(r) => self.clip = Some(*r),
                Draw::Unclip => self.clip = None,
            }
        }
    }

    pub fn crop(&self, r: [f32; 4]) -> Canvas {
        let (x0, y0) = (r[0].max(0.0) as usize, r[1].max(0.0) as usize);
        let (w, h) = ((r[2] as usize).min(self.w - x0), (r[3] as usize).min(self.h - y0));
        let mut out = Canvas::new(w, h, [0.0; 4]);
        for y in 0..h {
            for x in 0..w {
                out.px[y * w + x] = self.px[(y0 + y) * self.w + x0 + x];
            }
        }
        out
    }

    pub fn write_png(&self, path: &std::path::Path) {
        let mut buf = Vec::with_capacity(self.w * self.h * 4);
        for p in &self.px {
            for c in p {
                buf.push((c.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
        let file = std::fs::File::create(path).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), self.w as u32, self.h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&buf).unwrap();
    }
}
