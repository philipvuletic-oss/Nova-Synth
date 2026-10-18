//! Headless egui renderer: runs a UI function, tessellates it and rasterises the meshes on the
//! CPU, blending in gamma space with premultiplied alpha exactly like egui's glow backend.
//! It uses the egui version re-exported by `nih_plug_egui`, so no second egui is compiled.

pub mod nova_pages;

use nih_plug_egui::egui::{
    self, epaint::Primitive, ClippedPrimitive, Color32, TextureFilter, TextureId, ViewportId,
    ViewportInfo,
};
use std::collections::HashMap;
use std::path::Path;

/// A rendered RGBA8 image (opaque).
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn write_png(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(
            std::io::BufWriter::new(file),
            self.width as u32,
            self.height as u32,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(&self.rgba)
            .map_err(|e| e.to_string())
    }

    /// Number of distinct colours (capped), useful for "is this plausible" checks.
    pub fn distinct_colors(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        for px in self.rgba.chunks(4) {
            seen.insert([px[0], px[1], px[2]]);
            if seen.len() > 4096 {
                break;
            }
        }
        seen.len()
    }
}

struct Texture {
    size: [usize; 2],
    pixels: Vec<Color32>,
    nearest: bool,
}

/// Stateful headless egui host.
pub struct Renderer {
    pub ctx: egui::Context,
    textures: HashMap<TextureId, Texture>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            ctx: egui::Context::default(),
            textures: HashMap::new(),
        }
    }

    /// Runs `frames` frames (at least 2 so fonts and layout settle) and rasterises the last one.
    /// `width`/`height` are in physical pixels; the UI sees `width / pixels_per_point` points.
    pub fn render(
        &mut self,
        width: usize,
        height: usize,
        pixels_per_point: f32,
        frames: usize,
        mut ui: impl FnMut(&egui::Context),
    ) -> Image {
        let mut last = None;
        for _ in 0..frames.max(2) {
            let mut raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width as f32, height as f32) / pixels_per_point,
                )),
                ..Default::default()
            };
            raw.viewports.insert(
                ViewportId::ROOT,
                ViewportInfo {
                    native_pixels_per_point: Some(pixels_per_point),
                    ..Default::default()
                },
            );
            let output = self.ctx.run(raw, &mut ui);
            for (id, delta) in &output.textures_delta.set {
                self.apply_delta(*id, delta);
            }
            let ppp = output.pixels_per_point;
            let primitives = self.ctx.tessellate(output.shapes, ppp);
            for id in &output.textures_delta.free {
                self.textures.remove(id);
            }
            last = Some((primitives, ppp));
        }
        let (primitives, ppp) = last.expect("at least one frame is rendered");
        self.rasterize(width, height, ppp, &primitives)
    }

    fn apply_delta(&mut self, id: TextureId, delta: &egui::epaint::ImageDelta) {
        // Same conversion as egui_glow: font coverage -> premultiplied sRGBA with default gamma.
        let (size, pixels): ([usize; 2], Vec<Color32>) = match &delta.image {
            egui::ImageData::Color(image) => (image.size, image.pixels.clone()),
            egui::ImageData::Font(image) => (image.size, image.srgba_pixels(None).collect()),
        };
        let nearest = delta.options.magnification == TextureFilter::Nearest;
        match delta.pos {
            None => {
                self.textures.insert(
                    id,
                    Texture {
                        size,
                        pixels,
                        nearest,
                    },
                );
            }
            Some([x0, y0]) => {
                if let Some(texture) = self.textures.get_mut(&id) {
                    for y in 0..size[1] {
                        for x in 0..size[0] {
                            let (tx, ty) = (x0 + x, y0 + y);
                            if tx < texture.size[0] && ty < texture.size[1] {
                                texture.pixels[ty * texture.size[0] + tx] = pixels[y * size[0] + x];
                            }
                        }
                    }
                }
            }
        }
    }

    fn rasterize(
        &self,
        width: usize,
        height: usize,
        ppp: f32,
        primitives: &[ClippedPrimitive],
    ) -> Image {
        // Premultiplied RGBA in 0..1, gamma space (egui's non-sRGB-framebuffer blending).
        let mut frame = vec![[0.0f32, 0.0, 0.0, 1.0]; width * height];
        for primitive in primitives {
            let Primitive::Mesh(mesh) = &primitive.primitive else {
                continue;
            };
            let Some(texture) = self.textures.get(&mesh.texture_id) else {
                continue;
            };
            let clip = primitive.clip_rect;
            let clip_min_x = ((clip.min.x * ppp).round().max(0.0)) as usize;
            let clip_min_y = ((clip.min.y * ppp).round().max(0.0)) as usize;
            let clip_max_x = ((clip.max.x * ppp).round().clamp(0.0, width as f32)) as usize;
            let clip_max_y = ((clip.max.y * ppp).round().clamp(0.0, height as f32)) as usize;
            for tri in mesh.indices.chunks(3).filter(|tri| tri.len() == 3) {
                let v = [
                    mesh.vertices[tri[0] as usize],
                    mesh.vertices[tri[1] as usize],
                    mesh.vertices[tri[2] as usize],
                ];
                let p = [
                    (v[0].pos.x * ppp, v[0].pos.y * ppp),
                    (v[1].pos.x * ppp, v[1].pos.y * ppp),
                    (v[2].pos.x * ppp, v[2].pos.y * ppp),
                ];
                let area = edge(p[0], p[1], p[2]);
                if area.abs() < 1.0e-9 {
                    continue;
                }
                let min_x = p.iter().map(|q| q.0).fold(f32::MAX, f32::min).floor();
                let max_x = p.iter().map(|q| q.0).fold(f32::MIN, f32::max).ceil();
                let min_y = p.iter().map(|q| q.1).fold(f32::MAX, f32::min).floor();
                let max_y = p.iter().map(|q| q.1).fold(f32::MIN, f32::max).ceil();
                let x0 = (min_x.max(clip_min_x as f32)) as usize;
                let x1 = ((max_x.min(clip_max_x as f32)).max(0.0) as usize).min(width);
                let y0 = (min_y.max(clip_min_y as f32)) as usize;
                let y1 = ((max_y.min(clip_max_y as f32)).max(0.0) as usize).min(height);
                let colors = [v[0].color, v[1].color, v[2].color];
                for y in y0..y1 {
                    for x in x0..x1 {
                        let c = (x as f32 + 0.5, y as f32 + 0.5);
                        let w0 = edge(p[1], p[2], c) / area;
                        let w1 = edge(p[2], p[0], c) / area;
                        let w2 = edge(p[0], p[1], c) / area;
                        let eps = -1.0e-4;
                        if w0 < eps || w1 < eps || w2 < eps {
                            continue;
                        }
                        let u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                        let t = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                        let tex = sample(texture, u, t);
                        let mut vertex = [0.0f32; 4];
                        for (channel, out) in vertex.iter_mut().enumerate() {
                            let value = |color: Color32| color.to_array()[channel] as f32 / 255.0;
                            *out = w0 * value(colors[0])
                                + w1 * value(colors[1])
                                + w2 * value(colors[2]);
                        }
                        let src = [
                            tex[0] * vertex[0],
                            tex[1] * vertex[1],
                            tex[2] * vertex[2],
                            tex[3] * vertex[3],
                        ];
                        let dst = &mut frame[y * width + x];
                        let inv = 1.0 - src[3];
                        for channel in 0..4 {
                            dst[channel] = src[channel] + dst[channel] * inv;
                        }
                    }
                }
            }
        }
        let mut rgba = Vec::with_capacity(width * height * 4);
        for px in frame {
            for value in px.iter().take(3) {
                rgba.push((value.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            rgba.push(255);
        }
        Image {
            width,
            height,
            rgba,
        }
    }
}

fn edge(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn texel(texture: &Texture, x: i64, y: i64) -> [f32; 4] {
    let x = x.clamp(0, texture.size[0] as i64 - 1) as usize;
    let y = y.clamp(0, texture.size[1] as i64 - 1) as usize;
    let c = texture.pixels[y * texture.size[0] + x].to_array();
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        c[3] as f32 / 255.0,
    ]
}

fn sample(texture: &Texture, u: f32, v: f32) -> [f32; 4] {
    let fx = u * texture.size[0] as f32;
    let fy = v * texture.size[1] as f32;
    if texture.nearest {
        return texel(texture, fx.floor() as i64, fy.floor() as i64);
    }
    let (fx, fy) = (fx - 0.5, fy - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let a = texel(texture, x0, y0);
    let b = texel(texture, x0 + 1, y0);
    let c = texel(texture, x0, y0 + 1);
    let d = texel(texture, x0 + 1, y0 + 1);
    let mut out = [0.0; 4];
    for i in 0..4 {
        let top = a[i] + (b[i] - a[i]) * tx;
        let bottom = c[i] + (d[i] - c[i]) * tx;
        out[i] = top + (bottom - top) * ty;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_plain_egui_widgets() {
        let mut renderer = Renderer::new();
        let image = renderer.render(400, 200, 1.0, 3, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Hello headless");
                let _ = ui.button("A button");
                ui.label("Some body text");
            });
        });
        assert_eq!(image.rgba.len(), 400 * 200 * 4);
        // Background plus text/widget pixels means many distinct colours, not one flat fill.
        assert!(image.distinct_colors() > 20);
        let background = [image.rgba[0], image.rgba[1], image.rgba[2]];
        let text_pixels = image
            .rgba
            .chunks(4)
            .filter(|px| [px[0], px[1], px[2]] != background)
            .count();
        assert!(
            text_pixels > 500,
            "only {text_pixels} non-background pixels"
        );
    }

    #[test]
    fn scale_factor_doubles_output_resolution_content() {
        let mut renderer = Renderer::new();
        let image = renderer.render(800, 400, 2.0, 3, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Scaled");
            });
        });
        assert!(image.distinct_colors() > 5);
    }
}
