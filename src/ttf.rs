use super::Color;
use rusttype::{point, Font, Scale};

pub struct TtfFont<'a> {
    font: Font<'a>,
}

impl<'a> TtfFont<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, &'static str> {
        let font = Font::try_from_bytes(data).ok_or("Failed to parse TTF font")?;

        Ok(Self { font })
    }

    pub fn draw_text<F>(
        &self,
        text: &str,
        x: i32,
        y: i32,
        size: f32,
        color: Color,
        mut set_pixel: F,
    ) where
        F: FnMut(i32, i32, Color),
    {
        let scale = Scale::uniform(size);
        let v_metrics = self.font.v_metrics(scale);
        let baseline = y + v_metrics.ascent as i32;

        let mut cursor_x = x as f32;
        let mut cursor_y = baseline as f32;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = x as f32;
                cursor_y += v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
                continue;
            }

            let glyph = self.font.glyph(ch).scaled(scale);
            let h_metrics = glyph.h_metrics();
            let positioned = glyph.positioned(point(cursor_x, cursor_y));

            if let Some(bb) = positioned.pixel_bounding_box() {
                positioned.draw(|gx, gy, coverage| {
                    if coverage > 0.3 {
                        let px = bb.min.x + gx as i32;
                        let py = bb.min.y + gy as i32;
                        set_pixel(px, py, color);
                    }
                });
            }

            cursor_x += h_metrics.advance_width;
        }
    }

    pub fn draw_text_antialiased<F>(
        &self,
        text: &str,
        x: i32,
        y: i32,
        size: f32,
        color: Color,
        fb: &[u32],
        fb_width: u32,
        fb_height: u32,
        mut set_pixel: F,
    ) where
        F: FnMut(i32, i32, Color),
    {
        let scale = Scale::uniform(size);
        let v_metrics = self.font.v_metrics(scale);
        let baseline = y + v_metrics.ascent as i32;

        let mut cursor_x = x as f32;
        let mut cursor_y = baseline as f32;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = x as f32;
                cursor_y += v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
                continue;
            }

            let glyph = self.font.glyph(ch).scaled(scale);
            let h_metrics = glyph.h_metrics();
            let positioned = glyph.positioned(point(cursor_x, cursor_y));

            if let Some(bb) = positioned.pixel_bounding_box() {
                positioned.draw(|gx, gy, coverage| {
                    if coverage > 0.0 {
                        let px = bb.min.x + gx as i32;
                        let py = bb.min.y + gy as i32;

                        let bg = if px >= 0
                            && py >= 0
                            && px < fb_width as i32
                            && py < fb_height as i32
                        {
                            let idx = (py as u32 * fb_width + px as u32) as usize;
                            let pixel = fb[idx];
                            Color::rgba(
                                ((pixel >> 16) & 0xFF) as u8,
                                ((pixel >> 8) & 0xFF) as u8,
                                (pixel & 0xFF) as u8,
                                255,
                            )
                        } else {
                            Color::BLACK
                        };

                        let alpha = (coverage * 255.0) as u8;
                        let blended = Color::rgba(
                            ((color.r as u32 * alpha as u32 + bg.r as u32 * (255 - alpha) as u32)
                                / 255) as u8,
                            ((color.g as u32 * alpha as u32 + bg.g as u32 * (255 - alpha) as u32)
                                / 255) as u8,
                            ((color.b as u32 * alpha as u32 + bg.b as u32 * (255 - alpha) as u32)
                                / 255) as u8,
                            255,
                        );
                        set_pixel(px, py, blended);
                    }
                });
            }

            cursor_x += h_metrics.advance_width;
        }
    }

    pub fn draw_text_bold<F>(
        &self,
        text: &str,
        x: i32,
        y: i32,
        size: f32,
        color: Color,
        mut set_pixel: F,
    ) where
        F: FnMut(i32, i32, Color),
    {
        self.draw_text(text, x, y, size, color, &mut set_pixel);
        self.draw_text(text, x + 1, y, size, color, set_pixel);
    }

    pub fn text_width(&self, text: &str, size: f32) -> f32 {
        let scale = Scale::uniform(size);
        let mut width = 0.0;

        for ch in text.chars() {
            if ch == '\n' {
                continue;
            }

            let glyph = self.font.glyph(ch).scaled(scale);
            width += glyph.h_metrics().advance_width;
        }

        width
    }

    pub fn text_height(&self, size: f32) -> f32 {
        let scale = Scale::uniform(size);
        let v_metrics = self.font.v_metrics(scale);
        v_metrics.ascent - v_metrics.descent + v_metrics.line_gap
    }

    pub fn text_dimensions(&self, text: &str, size: f32) -> (f32, f32) {
        let scale = Scale::uniform(size);
        let v_metrics = self.font.v_metrics(scale);
        let line_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

        let mut max_width = 0.0;
        let mut current_width = 0.0;
        let mut line_count = 1;

        for ch in text.chars() {
            if ch == '\n' {
                if current_width > max_width {
                    max_width = current_width;
                }
                current_width = 0.0;
                line_count += 1;
            } else {
                let glyph = self.font.glyph(ch).scaled(scale);
                current_width += glyph.h_metrics().advance_width;
            }
        }

        if current_width > max_width {
            max_width = current_width;
        }

        (max_width, line_height * line_count as f32)
    }
}
