//! Drawing shapes into an image on the CPU, for "Copy PNG" and "Export PNG".
//!
//! egui's tessellator turns the shapes (boxes, lines, text) into anti-aliased triangles that
//! sample the font atlas. Filling those triangles here gives the same picture as on screen,
//! independent of the window, the scroll position and the platform.

use eframe::egui::{self, Color32, ColorImage, Shape, Vec2, epaint};

const MAX_SIDE: usize = 16384;

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
    let w = ((size.x * ppp).ceil() as usize).clamp(1, MAX_SIDE);
    let h = ((size.y * ppp).ceil() as usize).clamp(1, MAX_SIDE);
    let mut px = vec![rgba(bg); w * h];
    let tex = |u: f32, v: f32| -> [f32; 4] {
        let x = ((u * atlas.size[0] as f32) as usize).min(atlas.size[0] - 1);
        let y = ((v * atlas.size[1] as f32) as usize).min(atlas.size[1] - 1);
        rgba(atlas.pixels[y * atlas.size[0] + x])
    };
    for t in mesh.indices.as_chunks::<3>().0 {
        let v = t.map(|i| &mesh.vertices[i as usize]);
        let p = v.map(|v| [v.pos.x * ppp, v.pos.y * ppp]);
        let area =
            (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if area.abs() < 1e-6 {
            continue;
        }
        let (x0, x1) = pixel_span(p.map(|q| q[0]), w);
        let (y0, y1) = pixel_span(p.map(|q| q[1]), h);
        let col = v.map(|v| rgba(v.color));
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

fn rgba(c: Color32) -> [f32; 4] {
    [c.r() as f32, c.g() as f32, c.b() as f32, c.a() as f32]
}

fn pixel_span(coords: [f32; 3], len: usize) -> (usize, usize) {
    let min = coords.into_iter().fold(f32::MAX, f32::min);
    let max = coords.into_iter().fold(f32::MIN, f32::max);
    (
        min.floor().max(0.0) as usize,
        (max.ceil().max(0.0) as usize).min(len),
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Rect, pos2, vec2};

    fn rendered(shapes: Vec<Shape>, size: Vec2, ppp: f32, bg: Color32) -> ColorImage {
        let ctx = egui::Context::default();
        let mut img = None;
        let mut out = ctx.run_ui(Default::default(), |ui| {
            img = Some(render(ui.ctx(), shapes.clone(), size, ppp, bg));
        });
        out.textures_delta.clear();
        img.unwrap()
    }

    #[test]
    fn fills_rectangles_at_the_requested_scale() {
        let red = Color32::from_rgb(200, 0, 0);
        let rect = Rect::from_min_size(pos2(2.0, 1.0), vec2(4.0, 2.0));
        let img = rendered(
            vec![Shape::rect_filled(rect, 0.0, red)],
            vec2(8.0, 4.0),
            2.0,
            Color32::WHITE,
        );
        assert_eq!(img.size, [16, 8]);
        let at = |x: usize, y: usize| img.pixels[y * 16 + x];
        assert_eq!(at(0, 0), Color32::WHITE);
        assert_eq!(at(15, 7), Color32::WHITE);
        for (x, y) in [(6, 3), (10, 4), (5, 4)] {
            assert_eq!(at(x, y), red, "({x}, {y})");
        }
        assert_eq!(at(1, 3), Color32::WHITE);
        assert_eq!(at(8, 7), Color32::WHITE);
    }

    #[test]
    fn shapes_outside_the_picture_are_clipped() {
        let far = Rect::from_min_size(pos2(-50.0, -50.0), vec2(500.0, 500.0));
        let blue = Color32::from_rgb(0, 0, 255);
        let img = rendered(
            vec![Shape::rect_filled(far, 0.0, blue)],
            vec2(3.0, 2.0),
            1.0,
            Color32::WHITE,
        );
        assert_eq!(img.size, [3, 2]);
        assert!(img.pixels.iter().all(|p| *p == blue));
        let img = rendered(vec![], Vec2::ZERO, 1.0, Color32::BLACK);
        assert_eq!((img.size, img.pixels[0]), ([1, 1], Color32::BLACK));
    }

    #[test]
    fn encodes_png_that_decodes_to_the_same_pixels() {
        let img = ColorImage::new(
            [2, 2],
            vec![
                Color32::RED,
                Color32::from_rgb(1, 2, 3),
                Color32::WHITE,
                Color32::BLACK,
            ],
        );
        let bytes = encode(&img).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut buf = vec![0; 16];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!((info.width, info.height), (2, 2));
        assert_eq!(&buf[..8], &[255, 0, 0, 255, 1, 2, 3, 255]);
    }
}
