use super::Color;
use std::collections::BTreeMap;
use std::vec::Vec;

#[derive(Debug, Clone)]
pub enum FontType {
    BDF,
}

#[derive(Debug, Clone)]
pub struct Glyph {
    pub encoding: u32,
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub device_width: u32,
}

impl Glyph {
    pub fn draw<F>(&self, x: i32, y: i32, color: Color, mut set_pixel: F)
    where
        F: FnMut(i32, i32, Color),
    {
        let bytes_per_row = ((self.width + 7) / 8) as usize;

        for row in 0..self.height {
            let row_offset = row as usize * bytes_per_row;

            for col in 0..self.width {
                let byte_index = row_offset + (col / 8) as usize;
                let bit_index = 7 - (col % 8);

                let byte = self.bitmap[byte_index];

                if (byte & (1 << bit_index)) != 0 {
                    set_pixel(x + col as i32, y + row as i32, color);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Font {
    pub font_type: FontType,
    pub font_name: String,
    pub size: u32,
    pub bounding_box: (u32, u32, i32, i32),
    pub glyphs: BTreeMap<u32, Glyph>,
}

impl Font {
    pub fn new(font_type: FontType, font_name: String) -> Self {
        Self {
            font_type,
            font_name,
            size: 0,
            bounding_box: (0, 0, 0, 0),
            glyphs: BTreeMap::new(),
        }
    }

    pub fn get_glyph(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.get(&(ch as u32))
    }

    pub fn get_min_offsets(&self) -> (i32, i32) {
        let mut min_x = 0;
        let mut min_y = 0;

        for glyph in self.glyphs.values() {
            if glyph.offset_x < min_x {
                min_x = glyph.offset_x;
            }
            if glyph.offset_y < min_y {
                min_y = glyph.offset_y;
            }
        }

        (min_x, min_y)
    }

    pub fn draw_char<F>(&self, ch: char, x: i32, y: i32, color: Color, set_pixel: F) -> i32
    where
        F: FnMut(i32, i32, Color),
    {
        if let Some(glyph) = self.get_glyph(ch) {
            glyph.draw(x, y, color, set_pixel);
            glyph.width as i32
        } else {
            self.bounding_box.0 as i32
        }
    }

    pub fn draw_text<F>(&self, text: &str, x: i32, y: i32, color: Color, mut set_pixel: F)
    where
        F: FnMut(i32, i32, Color),
    {
        let line_height = self.text_height() as i32;
        let mut current_x = x;
        let mut current_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                current_x = x;
                current_y += line_height;
            } else {
                let advance = self.draw_char(ch, current_x, current_y, color, &mut set_pixel);
                current_x += advance;
            }
        }
    }

    pub fn text_width(&self, text: &str) -> u32 {
        let mut max_width = 0;
        let mut current_line_width = 0;

        for ch in text.chars() {
            if ch == '\n' {
                if current_line_width > max_width {
                    max_width = current_line_width;
                }
                current_line_width = 0;
            } else {
                if let Some(glyph) = self.get_glyph(ch) {
                    current_line_width += glyph.device_width;
                } else {
                    current_line_width += self.bounding_box.0;
                }
            }
        }

        if current_line_width > max_width {
            max_width = current_line_width;
        }

        max_width
    }

    pub fn text_height(&self) -> u32 {
        self.bounding_box.1
    }

    pub fn text_dimensions(&self, text: &str) -> (u32, u32) {
        let width = self.text_width(text);
        let line_count = text.chars().filter(|&c| c == '\n').count() + 1;
        let height = self.text_height() * line_count as u32;
        (width, height)
    }
}

pub fn parse_bdf_font(data: &[u8]) -> Result<Font, &'static str> {
    let mut font = Font {
        font_type: FontType::BDF,
        font_name: String::new(),
        size: 0,
        bounding_box: (0, 0, 0, 0),
        glyphs: BTreeMap::new(),
    };

    let mut current_glyph: Option<Glyph> = None;
    let mut in_bitmap = false;
    let mut bitmap_data: Vec<u8> = Vec::new();

    let mut line_start = 0;
    let bytes = data;

    for i in 0..=bytes.len() {
        if i == bytes.len() || bytes[i] == b'\n' {
            if i > line_start {
                let line_bytes = &bytes[line_start..i];
                let line = core::str::from_utf8(line_bytes).unwrap_or("");
                let line = line.trim();

                if !line.is_empty() {
                    parse_line(
                        line,
                        &mut font,
                        &mut current_glyph,
                        &mut in_bitmap,
                        &mut bitmap_data,
                    );
                }
            }
            line_start = i + 1;
        }
    }

    Ok(font)
}

fn parse_line(
    line: &str,
    font: &mut Font,
    current_glyph: &mut Option<Glyph>,
    in_bitmap: &mut bool,
    bitmap_data: &mut Vec<u8>,
) {
    if line.starts_with("FONT ") {
        font.font_name = line[5..].trim().into();
    } else if line.starts_with("SIZE ") {
        font.size = line[5..].trim().parse().unwrap_or(0);
    } else if line.starts_with("FONTBOUNDINGBOX ") {
        let parts: Vec<_> = line[16..].split_whitespace().collect();
        if parts.len() >= 4 {
            font.bounding_box = (
                parts[0].parse().unwrap_or(0),
                parts[1].parse().unwrap_or(0),
                parts[2].parse().unwrap_or(0),
                parts[3].parse().unwrap_or(0),
            );
        }
    } else if line.starts_with("STARTCHAR") {
        *current_glyph = Some(Glyph {
            encoding: 0,
            bitmap: Vec::new(),
            width: 0,
            height: 0,
            offset_x: 0,
            offset_y: 0,
            device_width: 0,
        });
    } else if line.starts_with("ENCODING ") {
        if let Some(ref mut glyph) = current_glyph {
            glyph.encoding = line[9..].trim().parse().unwrap_or(0);
        }
    } else if line.starts_with("DWIDTH ") {
        if let Some(ref mut glyph) = current_glyph {
            let parts: Vec<_> = line[7..].split_whitespace().collect();
            if !parts.is_empty() {
                glyph.device_width = parts[0].parse().unwrap_or(0);
            }
        }
    } else if line.starts_with("BBX ") {
        if let Some(ref mut glyph) = current_glyph {
            let parts: Vec<_> = line[4..].split_whitespace().collect();
            if parts.len() >= 4 {
                glyph.width = parts[0].parse().unwrap_or(0);
                glyph.height = parts[1].parse().unwrap_or(0);
                glyph.offset_x = parts[2].parse().unwrap_or(0);
                glyph.offset_y = parts[3].parse().unwrap_or(0);
            }
        }
    } else if line == "BITMAP" {
        *in_bitmap = true;
        bitmap_data.clear();
    } else if line == "ENDCHAR" {
        if let Some(mut glyph) = current_glyph.take() {
            if glyph.encoding < 256 {
                glyph.bitmap = core::mem::take(bitmap_data);
                font.glyphs.insert(glyph.encoding, glyph);
            }
        }
        *in_bitmap = false;
    } else if *in_bitmap {
        let hex_str = line.trim();
        for i in (0..hex_str.len()).step_by(2) {
            let end = (i + 2).min(hex_str.len());
            if let Ok(byte) = u8::from_str_radix(&hex_str[i..end], 16) {
                bitmap_data.push(byte);
            }
        }
    }
}
