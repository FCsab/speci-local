//! Drawing shapes into an image on the CPU, for "Copy PNG" and "Export PNG".
//!
//! egui's tessellator turns the shapes (boxes, lines, text) into anti-aliased triangles that
//! sample the font atlas. Filling those triangles here gives the same picture as on screen,
//! independent of the window, the scroll position and the platform.

use eframe::egui::{self, Color32, ColorImage, Shape, Vec2, epaint};

/// Render `shapes` (in points, starting at 0,0) at `ppp` pixels per point over `bg`.
pub fn render(
    ctx: &egui::Context,
    shapes: Vec<Shape>,
    size: Vec2,
    ppp: f32,
    bg: Color32,
) -> ColorImage {
    let (atlas, atlas_size) = ctx.fonts(|f| (f.image(), f.font_image_size()));
    let options = epaint::TessellationOptions {
        round_text_to_pixels: true,
        ..Default::default()
    };
    let mut tess = epaint::Tessellator::new(ppp, options, atlas_size, vec![]);
    let mut mesh = epaint::Mesh::default();
    for s in shapes {
        tess.tessellate_shape(s, &mut mesh);
    }
    let w = ((size.x * ppp).ceil() as usize).clamp(1, 16384);
    let h = ((size.y * ppp).ceil() as usize).clamp(1, 16384);
    let bgf = [bg.r() as f32, bg.g() as f32, bg.b() as f32, bg.a() as f32];
    let mut px = vec![bgf; w * h];
    let tex = |u: f32, v: f32| -> [f32; 4] {
        let x = ((u * atlas.size[0] as f32) as usize).min(atlas.size[0] - 1);
        let y = ((v * atlas.size[1] as f32) as usize).min(atlas.size[1] - 1);
        let c = atlas.pixels[y * atlas.size[0] + x];
        [c.r() as f32, c.g() as f32, c.b() as f32, c.a() as f32]
    };
    for t in mesh.indices.chunks_exact(3) {
        let v = [
            &mesh.vertices[t[0] as usize],
            &mesh.vertices[t[1] as usize],
            &mesh.vertices[t[2] as usize],
        ];
        let p: Vec<[f32; 2]> = v.iter().map(|v| [v.pos.x * ppp, v.pos.y * ppp]).collect();
        let area =
            (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if area.abs() < 1e-6 {
            continue;
        }
        let x0 = p
            .iter()
            .map(|q| q[0])
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let x1 = (p
            .iter()
            .map(|q| q[0])
            .fold(f32::MIN, f32::max)
            .ceil()
            .max(0.0) as usize)
            .min(w);
        let y0 = p
            .iter()
            .map(|q| q[1])
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as usize;
        let y1 = (p
            .iter()
            .map(|q| q[1])
            .fold(f32::MIN, f32::max)
            .ceil()
            .max(0.0) as usize)
            .min(h);
        let col: Vec<[f32; 4]> = v
            .iter()
            .map(|v| {
                [
                    v.color.r() as f32,
                    v.color.g() as f32,
                    v.color.b() as f32,
                    v.color.a() as f32,
                ]
            })
            .collect();
        for y in y0..y1 {
            for x in x0..x1 {
                let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                let edge = |a: [f32; 2], b: [f32; 2]| {
                    (b[0] - a[0]) * (cy - a[1]) - (b[1] - a[1]) * (cx - a[0])
                };
                let w0 = edge(p[1], p[2]) / area;
                let w1 = edge(p[2], p[0]) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                    continue;
                }
                let u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                let vv = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                let t = tex(u, vv);
                let mut src = [0.0; 4];
                for i in 0..4 {
                    // premultiplied vertex color × premultiplied texel
                    src[i] = (w0 * col[0][i] + w1 * col[1][i] + w2 * col[2][i]) * t[i] / 255.0;
                }
                let d = &mut px[y * w + x];
                let k = 1.0 - src[3] / 255.0;
                for i in 0..4 {
                    d[i] = src[i] + d[i] * k;
                }
            }
        }
    }
    let pixels = px
        .into_iter()
        .map(|c| {
            let q = |x: f32| x.round().clamp(0.0, 255.0) as u8;
            Color32::from_rgba_premultiplied(q(c[0]), q(c[1]), q(c[2]), q(c[3]))
        })
        .collect();
    ColorImage::new([w, h], pixels)
}

/// PNG file bytes of an image.
pub fn encode(img: &ColorImage) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.size[0] as u32, img.size[1] as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        let data: Vec<u8> = img
            .pixels
            .iter()
            .flat_map(|c| c.to_srgba_unmultiplied())
            .collect();
        w.write_image_data(&data).map_err(|e| e.to_string())?;
    }
    Ok(out)
}
