use super::Color;
use std::collections::BTreeMap;
use std::vec::Vec;

#[derive(Debug, Clone)]
pub enum FontType {
    BDF,
    TTF,
}

pub struct TtfFont<'a> {
    font_data: &'a [u8],
    cache: BTreeMap<(u32, u32), CachedGlyph>,
}

#[derive(Clone)]
struct CachedGlyph {
    bitmap: Vec<u8>,
    width: u32,
    height: u32,
    bearing_x: i32,
    bearing_y: i32,
    advance: u32,
}

impl<'a> TtfFont<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, &'static str> {
        use ttf_parser::Face;

        Face::parse(data, 0).map_err(|_| "Invalid TTF file")?;

        Ok(Self {
            font_data: data,
            cache: BTreeMap::new(),
        })
    }

    fn rasterize_glyph(&mut self, ch: char, size: u32) -> Option<CachedGlyph> {
        use ab_glyph_rasterizer::Rasterizer;
        use ttf_parser::Face;

        let face = Face::parse(self.font_data, 0).ok()?;
        let glyph_id = face.glyph_index(ch)?;

        let units_per_em = face.units_per_em() as f32;
        let scale = size as f32 / units_per_em;

        let h_advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * scale;
        let bbox = face.glyph_bounding_box(glyph_id)?;

        let bearing_x = (bbox.x_min as f32 * scale) as i32;
        let bearing_y = (bbox.y_max as f32 * scale) as i32;
        let width = ((bbox.x_max - bbox.x_min) as f32 * scale).ceil() as u32;
        let height = ((bbox.y_max - bbox.y_min) as f32 * scale).ceil() as u32;

        if width == 0 || height == 0 {
            return Some(CachedGlyph {
                bitmap: Vec::new(),
                width: 0,
                height: 0,
                bearing_x,
                bearing_y,
                advance: h_advance as u32,
            });
        }

        let mut builder = OutlineConverter::new(scale, bearing_x as f32, bearing_y as f32);
        face.outline_glyph(glyph_id, &mut builder)?;

        let mut rasterizer = Rasterizer::new(width as usize, height as usize);

        for contour in &builder.contours {
            if contour.is_empty() {
                continue;
            }

            let mut i = 0;
            while i < contour.len() {
                let p0 = contour[i];
                let p1 = if i + 1 < contour.len() {
                    contour[i + 1]
                } else {
                    contour[0]
                };

                rasterizer.draw_line(p0, p1);
                i += 1;
            }
        }

        let mut bitmap = vec![0u8; (width * height) as usize];
        rasterizer.for_each_pixel(|index, alpha| {
            if index < bitmap.len() {
                bitmap[index] = (alpha * 255.0) as u8;
            }
        });

        Some(CachedGlyph {
            bitmap,
            width,
            height,
            bearing_x,
            bearing_y,
            advance: h_advance as u32,
        })
    }

    pub fn draw_text<F>(
        &mut self,
        text: &str,
        x: i32,
        y: i32,
        size: u32,
        color: Color,
        mut set_pixel: F,
    ) where
        F: FnMut(i32, i32, Color),
    {
        let mut current_x = x;
        let mut current_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                current_x = x;
                current_y += size as i32;
                continue;
            }

            let key = (ch as u32, size);

            if !self.cache.contains_key(&key) {
                if let Some(glyph) = self.rasterize_glyph(ch, size) {
                    self.cache.insert(key, glyph);
                }
            }

            if let Some(glyph) = self.cache.get(&key) {
                for row in 0..glyph.height {
                    for col in 0..glyph.width {
                        let idx = (row * glyph.width + col) as usize;
                        let alpha = glyph.bitmap[idx];

                        if alpha > 0 {
                            let px = current_x + col as i32 + glyph.bearing_x;
                            let py = current_y - glyph.bearing_y + row as i32;

                            if alpha > 128 {
                                set_pixel(px, py, color);
                            }
                        }
                    }
                }

                current_x += glyph.advance as i32;
            }
        }
    }

    pub fn draw_text_bold<F>(
        &mut self,
        text: &str,
        x: i32,
        y: i32,
        size: u32,
        color: Color,
        mut set_pixel: F,
    ) where
        F: FnMut(i32, i32, Color),
    {
        self.draw_text(text, x, y, size, color, &mut set_pixel);
        self.draw_text(text, x + 1, y, size, color, set_pixel);
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn text_width(&mut self, text: &str, size: u32) -> u32 {
        let mut width = 0;

        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }

            let key = (ch as u32, size);

            if !self.cache.contains_key(&key) {
                if let Some(glyph) = self.rasterize_glyph(ch, size) {
                    self.cache.insert(key, glyph);
                }
            }

            if let Some(glyph) = self.cache.get(&key) {
                width += glyph.advance;
            }
        }

        width
    }
}

struct OutlineConverter {
    contours: Vec<Vec<ab_glyph_rasterizer::Point>>,
    current: Vec<ab_glyph_rasterizer::Point>,
    current_point: Option<ab_glyph_rasterizer::Point>,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

impl OutlineConverter {
    fn new(scale: f32, offset_x: f32, offset_y: f32) -> Self {
        Self {
            contours: Vec::new(),
            current: Vec::new(),
            current_point: None,
            scale,
            offset_x,
            offset_y,
        }
    }

    fn transform(&self, x: f32, y: f32) -> ab_glyph_rasterizer::Point {
        ab_glyph_rasterizer::point(
            x * self.scale - self.offset_x,
            self.offset_y - y * self.scale,
        )
    }
}

impl ttf_parser::OutlineBuilder for OutlineConverter {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.current.is_empty() {
            self.contours.push(core::mem::take(&mut self.current));
        }
        let p = self.transform(x, y);
        self.current.push(p);
        self.current_point = Some(p);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.transform(x, y);
        self.current.push(p);
        self.current_point = Some(p);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        if let Some(p0) = self.current_point {
            let p1 = self.transform(x1, y1);
            let p2 = self.transform(x, y);

            let steps = 10;
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let t2 = t * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;

                let px = p0.x * mt2 + 2.0 * p1.x * mt * t + p2.x * t2;
                let py = p0.y * mt2 + 2.0 * p1.y * mt * t + p2.y * t2;

                self.current.push(ab_glyph_rasterizer::point(px, py));
            }
            self.current_point = Some(p2);
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        if let Some(p0) = self.current_point {
            let p1 = self.transform(x1, y1);
            let p2 = self.transform(x2, y2);
            let p3 = self.transform(x, y);

            let steps = 15;
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let t2 = t * t;
                let t3 = t2 * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;
                let mt3 = mt2 * mt;

                let px = p0.x * mt3 + 3.0 * p1.x * mt2 * t + 3.0 * p2.x * mt * t2 + p3.x * t3;
                let py = p0.y * mt3 + 3.0 * p1.y * mt2 * t + 3.0 * p2.y * mt * t2 + p3.y * t3;

                self.current.push(ab_glyph_rasterizer::point(px, py));
            }
            self.current_point = Some(p3);
        }
    }

    fn close(&mut self) {
        if !self.current.is_empty() {
            self.contours.push(core::mem::take(&mut self.current));
        }
        self.current_point = None;
    }
}
