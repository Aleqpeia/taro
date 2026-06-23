//! Procedurally generated textures — no external art needed, so they stay
//! self-contained and render fine under software rasterization.
//!
//! Everything here writes straight RGBA8 (sRGB) byte buffers and wraps them in
//! a Bevy [`Image`]. Three pieces:
//!   * [`vignette_image`] — the felt background with a warm center glow and
//!     darkened edges, so the table reads as lit from above.
//!   * [`soft_shadow_image`] — a reusable soft-edged dark blob placed under each
//!     card for depth.
//!   * [`card_back_image`] — the ornamental Tarot card back (deep indigo, a
//!     double gold frame, and a single central star motif). Restrained on
//!     purpose: a clean back beats a busy one.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// A tiny RGBA8 canvas with alpha-blended drawing helpers.
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Self { w, h, px: vec![0; w * h * 4] }
    }

    /// Source-over alpha blend of `(r,g,b)` at coverage `a` (0..=1) onto a pixel.
    fn blend(&mut self, x: usize, y: usize, r: f32, g: f32, b: f32, a: f32) {
        if x >= self.w || y >= self.h || a <= 0.0 {
            return;
        }
        let a = a.clamp(0.0, 1.0);
        let i = (y * self.w + x) * 4;
        let bg = &self.px[i..i + 4];
        let (br, bg_, bb, ba) = (
            bg[0] as f32 / 255.0,
            bg[1] as f32 / 255.0,
            bg[2] as f32 / 255.0,
            bg[3] as f32 / 255.0,
        );
        let out_a = a + ba * (1.0 - a);
        let mix = |s: f32, d: f32| {
            if out_a <= 0.0 {
                0.0
            } else {
                (s * a + d * ba * (1.0 - a)) / out_a
            }
        };
        self.px[i] = (mix(r, br) * 255.0).round() as u8;
        self.px[i + 1] = (mix(g, bg_) * 255.0).round() as u8;
        self.px[i + 2] = (mix(b, bb) * 255.0).round() as u8;
        self.px[i + 3] = (out_a * 255.0).round() as u8;
    }

    /// Fill the whole canvas with an opaque colour.
    fn fill(&mut self, r: f32, g: f32, b: f32) {
        for y in 0..self.h {
            for x in 0..self.w {
                self.blend(x, y, r, g, b, 1.0);
            }
        }
    }

    fn into_image(self) -> Image {
        Image::new(
            Extent3d { width: self.w as u32, height: self.h as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            self.px,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Signed coverage of a rounded rectangle centred in `(w,h)`: 1 inside, a soft
/// edge across `feather` pixels, 0 outside. `inset` shrinks the rect from the
/// canvas edge; `radius` rounds the corners.
fn rounded_rect_coverage(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    inset: f32,
    radius: f32,
    feather: f32,
) -> f32 {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let half_w = (w / 2.0 - inset - radius).max(0.0);
    let half_h = (h / 2.0 - inset - radius).max(0.0);
    let dx = (x - cx).abs() - half_w;
    let dy = (y - cy).abs() - half_h;
    // Distance to the rounded-rect boundary (negative inside).
    let outside = ((dx.max(0.0)).powi(2) + (dy.max(0.0)).powi(2)).sqrt();
    let inside = dx.max(dy).min(0.0);
    let dist = outside + inside - radius;
    1.0 - smoothstep(-feather, feather, dist)
}

/// The felt background: a warm pool of light at the centre fading to a dark,
/// slightly cool vignette at the edges.
pub fn vignette_image(w: usize, h: usize) -> Image {
    let mut c = Canvas::new(w, h);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let max_r = (cx * cx + cy * cy).sqrt();

    // Base felt + radial gradient (center -> edge).
    let center = (0.115_f32, 0.085, 0.155); // lifted indigo, lit
    let edge = (0.030_f32, 0.024, 0.052); // deep shadow
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let r = (dx * dx + dy * dy).sqrt() / max_r;
            // Ease the falloff so the lit pool is broad and the corners drop off.
            let t = smoothstep(0.0, 1.05, r).powf(1.3);
            let rr = center.0 + (edge.0 - center.0) * t;
            let gg = center.1 + (edge.1 - center.1) * t;
            let bb = center.2 + (edge.2 - center.2) * t;
            c.blend(x, y, rr, gg, bb, 1.0);
        }
    }
    c.into_image()
}

/// A reusable soft shadow: a dark, blurred rounded rectangle on transparency.
/// Drawn under a card (scaled to its size) to lift it off the table.
pub fn soft_shadow_image(size: usize) -> Image {
    let mut c = Canvas::new(size, size);
    let s = size as f32;
    for y in 0..size {
        for x in 0..size {
            // A generously feathered rounded rect, inset so the blur has room.
            let cov = rounded_rect_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                s,
                s,
                s * 0.16,
                s * 0.10,
                s * 0.16,
            );
            // Slightly more than coverage near the core for a denser center.
            let a = (cov * 0.55).clamp(0.0, 0.55);
            c.blend(x, y, 0.0, 0.0, 0.0, a);
        }
    }
    c.into_image()
}

/// A white, soft-edged rounded glow — tinted at the sprite to colour it (e.g.
/// the gold selection halo behind the active card).
pub fn glow_image(size: usize) -> Image {
    let mut c = Canvas::new(size, size);
    let s = size as f32;
    for y in 0..size {
        for x in 0..size {
            let cov = rounded_rect_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                s,
                s,
                s * 0.22,
                s * 0.14,
                s * 0.20,
            );
            if cov > 0.0 {
                c.blend(x, y, 1.0, 1.0, 1.0, cov);
            }
        }
    }
    c.into_image()
}

/// The reading-panel background: a translucent dark rounded card with a thin
/// gold hairline border.
pub fn panel_image(w: usize, h: usize) -> Image {
    let mut c = Canvas::new(w, h);
    let fw = w as f32;
    let fh = h as f32;
    let radius = 22.0;
    let gold = (0.72_f32, 0.60, 0.34);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let fill = rounded_rect_coverage(fx, fy, fw, fh, 1.0, radius, 1.2);
            if fill > 0.0 {
                c.blend(x, y, 0.085, 0.068, 0.125, fill * 0.92);
            }
            // Gold hairline just inside the edge.
            let outer = rounded_rect_coverage(fx, fy, fw, fh, 2.0, radius, 0.8);
            let inner = rounded_rect_coverage(fx, fy, fw, fh, 3.6, radius, 0.8);
            let band = (outer - inner).clamp(0.0, 1.0);
            if band > 0.0 {
                c.blend(x, y, gold.0, gold.1, gold.2, band * 0.6);
            }
        }
    }
    c.into_image()
}

/// A small badge disc for position numbers: a dark fill ringed in gold.
pub fn disc_image(size: usize) -> Image {
    let mut c = Canvas::new(size, size);
    let s = size as f32;
    let cx = s / 2.0;
    let cy = s / 2.0;
    let r = s / 2.0 - 1.0;
    let ring = s * 0.10;
    let gold = (0.82_f32, 0.68, 0.36);
    let fill = (0.08_f32, 0.06, 0.11);
    for y in 0..size {
        for x in 0..size {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let disc = 1.0 - smoothstep(r - 1.0, r + 0.5, d);
            if disc <= 0.0 {
                continue;
            }
            // Gold near the rim, dark fill inside.
            let is_ring = smoothstep(r - ring - 1.0, r - ring + 1.0, d);
            let rr = fill.0 + (gold.0 - fill.0) * is_ring;
            let gg = fill.1 + (gold.1 - fill.1) * is_ring;
            let bb = fill.2 + (gold.2 - fill.2) * is_ring;
            c.blend(x, y, rr, gg, bb, disc);
        }
    }
    c.into_image()
}

/// The ornamental card back. Deep indigo field, a faint diagonal lattice, a
/// double gold frame, and a single eight-point star at the centre.
pub fn card_back_image(w: usize, h: usize) -> Image {
    let mut c = Canvas::new(w, h);
    let fw = w as f32;
    let fh = h as f32;

    // Field: deep indigo, faintly darker toward the edges.
    let field = (0.105_f32, 0.090, 0.205);
    c.fill(field.0, field.1, field.2);

    // Faint diagonal lattice for texture (very low alpha gold).
    let gold = (0.82_f32, 0.68, 0.36);
    let spacing = (fw / 7.0).max(8.0);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32;
            let fy = y as f32;
            let a = (((fx + fy) / spacing * std::f32::consts::PI).sin().abs()).powf(8.0)
                + (((fx - fy) / spacing * std::f32::consts::PI).sin().abs()).powf(8.0);
            if a > 0.02 {
                c.blend(x, y, gold.0, gold.1, gold.2, a * 0.05);
            }
        }
    }

    // Mask the lattice/field to a rounded-rect card shape (transparent corners).
    let radius = fw * 0.07;
    for y in 0..h {
        for x in 0..w {
            let cov = rounded_rect_coverage(
                x as f32 + 0.5,
                y as f32 + 0.5,
                fw,
                fh,
                0.0,
                radius,
                1.2,
            );
            if cov < 0.999 {
                let i = (y * w + x) * 4;
                c.px[i + 3] = (c.px[i + 3] as f32 * cov) as u8;
            }
        }
    }

    // Double gold frame (outer thick, inner thin), following the rounded shape.
    let draw_frame = |c: &mut Canvas, inset: f32, thickness: f32, alpha: f32| {
        for y in 0..h {
            for x in 0..w {
                let fx = x as f32 + 0.5;
                let fy = y as f32 + 0.5;
                let outer =
                    rounded_rect_coverage(fx, fy, fw, fh, inset, radius * 0.8, 0.8);
                let inner = rounded_rect_coverage(
                    fx,
                    fy,
                    fw,
                    fh,
                    inset + thickness,
                    radius * 0.8,
                    0.8,
                );
                let band = (outer - inner).clamp(0.0, 1.0);
                if band > 0.0 {
                    c.blend(x, y, gold.0, gold.1, gold.2, band * alpha);
                }
            }
        }
    };
    draw_frame(&mut c, fw * 0.06, fw * 0.018, 0.95);
    draw_frame(&mut c, fw * 0.11, fw * 0.010, 0.75);

    // Central eight-point star: two overlaid squares (a square + its 45° twin),
    // rendered as a filled star polygon via angular radius modulation.
    let cx = fw / 2.0;
    let cy = fh / 2.0;
    let r_out = fw * 0.20;
    let r_in = r_out * 0.42;
    let points = 8.0;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let ang = dy.atan2(dx);
            // Star radius at this angle (cusped between r_in and r_out).
            let phase = (ang * points / 2.0).cos().abs();
            let r_edge = r_in + (r_out - r_in) * phase;
            let cov = 1.0 - smoothstep(r_edge - 1.2, r_edge + 1.2, dist);
            if cov > 0.0 {
                c.blend(x, y, gold.0, gold.1, gold.2, cov * 0.92);
            }
        }
    }
    // A small dark center dot inside the star for definition.
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let cov = 1.0 - smoothstep(r_in * 0.5 - 1.0, r_in * 0.5 + 1.0, dist);
            if cov > 0.0 {
                c.blend(x, y, field.0, field.1, field.2, cov);
            }
        }
    }

    c.into_image()
}
