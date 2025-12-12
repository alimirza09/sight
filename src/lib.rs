#![allow(dead_code)]
use minifb::{Scale, ScaleMode, Window, WindowOptions};

pub mod bdf;
pub mod bmp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub fn to_u32(self) -> u32 {
        (self.b as u32) | ((self.g as u32) << 8) | ((self.r as u32) << 16) | ((self.a as u32) << 24)
    }
    pub fn blend(self, bg: Color) -> Color {
        let a = self.a as u32;
        if a == 255 {
            return self;
        }
        if a == 0 {
            return bg;
        }
        let inv = 255 - a;
        Color {
            r: ((self.r as u32 * a + bg.r as u32 * inv) / 255) as u8,
            g: ((self.g as u32 * a + bg.g as u32 * inv) / 255) as u8,
            b: ((self.b as u32 * a + bg.b as u32 * inv) / 255) as u8,
            a: 255,
        }
    }
    pub fn lerp(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            g: (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            b: (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
            a: (self.a as f32 + (other.a as f32 - self.a as f32) * t) as u8,
        }
    }
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub struct Sight {
    buffer: Vec<u32>,
    window: Window,
    width: u32,
    height: u32,
    dirty: bool,
}

impl Sight {
    pub fn new() -> Result<Self, &'static str> {
        let width = 800;
        let height = 600;
        let buffer = vec![0u32; (width * height) as usize];
        let window = Window::new(
            "Sight ported to minifb",
            width as usize,
            height as usize,
            WindowOptions {
                resize: true,
                scale: Scale::X2,
                scale_mode: ScaleMode::AspectRatioStretch,
                ..Default::default()
            },
        )
        .map_err(|_| "Failed to create window")?;
        Ok(Self {
            buffer,
            window,
            width,
            height,
            dirty: true,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn clear(&mut self, color: Color) {
        self.buffer.fill(color.to_u32());
        self.dirty = true;
    }

    pub fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        self.buffer[idx] = color.to_u32();
        self.dirty = true;
    }

    fn put_pixel_aa(&mut self, x: i32, y: i32, color: Color, _alpha: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        let existing = self.buffer[idx];
        let existing = Color {
            b: (existing & 0xFF) as u8,
            g: ((existing >> 8) & 0xFF) as u8,
            r: ((existing >> 16) & 0xFF) as u8,
            a: 255,
        };
        let blended = color.blend(existing);
        self.buffer[idx] = blended.to_u32();
        self.dirty = true;
    }

    pub fn draw_line(&mut self, p1: Point, p2: Point, color: Color) {
        let mut x0 = p1.x as f32;
        let mut y0 = p1.y as f32;
        let mut x1 = p2.x as f32;
        let mut y1 = p2.y as f32;
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            core::mem::swap(&mut x0, &mut y0);
            core::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            core::mem::swap(&mut x0, &mut x1);
            core::mem::swap(&mut y0, &mut y1);
        }
        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx == 0.0 { 1.0 } else { dy / dx };
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - (x0 + 0.5).fract();
        let xpxl1 = xend as i32;
        let ypxl1 = yend.floor() as i32;
        if steep {
            self.put_pixel_aa(ypxl1, xpxl1, color, (1.0 - yend.fract()) * xgap);
            self.put_pixel_aa(ypxl1 + 1, xpxl1, color, yend.fract() * xgap);
        } else {
            self.put_pixel_aa(xpxl1, ypxl1, color, (1.0 - yend.fract()) * xgap);
            self.put_pixel_aa(xpxl1, ypxl1 + 1, color, yend.fract() * xgap);
        }
        let mut intery = yend + gradient;
        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as i32;
        let ypxl2 = yend.floor() as i32;
        if steep {
            self.put_pixel_aa(ypxl2, xpxl2, color, (1.0 - yend.fract()) * xgap);
            self.put_pixel_aa(ypxl2 + 1, xpxl2, color, yend.fract() * xgap);
        } else {
            self.put_pixel_aa(xpxl2, ypxl2, color, (1.0 - yend.fract()) * xgap);
            self.put_pixel_aa(xpxl2, ypxl2 + 1, color, yend.fract() * xgap);
        }
        for x in (xpxl1 + 1)..xpxl2 {
            let y = intery.floor() as i32;
            let frac = intery.fract();
            if steep {
                self.put_pixel_aa(y, x, color, 1.0 - frac);
                self.put_pixel_aa(y + 1, x, color, frac);
            } else {
                self.put_pixel_aa(x, y, color, 1.0 - frac);
                self.put_pixel_aa(x, y + 1, color, frac);
            }
            intery += gradient;
        }
    }

    pub fn draw_thick_line(&mut self, p1: Point, p2: Point, thickness: u32, color: Color) {
        let dx = (p2.x - p1.x) as f32;
        let dy = (p2.y - p1.y) as f32;
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            return;
        }
        let nx = -dy / len;
        let ny = dx / len;
        let half = thickness as f32 / 2.0;
        for i in 0..thickness {
            let offset = i as f32 - half;
            let start = Point::new(
                (p1.x as f32 + nx * offset) as i32,
                (p1.y as f32 + ny * offset) as i32,
            );
            let end = Point::new(
                (p2.x as f32 + nx * offset) as i32,
                (p2.y as f32 + ny * offset) as i32,
            );
            self.draw_line(start, end, color);
        }
    }

    pub fn draw_rect(&mut self, r: Rect, color: Color) {
        let p1 = Point::new(r.x, r.y);
        let p2 = Point::new(r.x + r.width as i32, r.y);
        let p3 = Point::new(r.x + r.width as i32, r.y + r.height as i32);
        let p4 = Point::new(r.x, r.y + r.height as i32);
        self.draw_line(p1, p2, color);
        self.draw_line(p2, p3, color);
        self.draw_line(p3, p4, color);
        self.draw_line(p4, p1, color);
    }

    pub fn fill_rect(&mut self, r: Rect, color: Color) {
        for y in r.y..(r.y + r.height as i32) {
            for x in r.x..(r.x + r.width as i32) {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn draw_circle(&mut self, center: Point, radius: u32, color: Color) {
        let r = radius as i32;
        let mut x = 0;
        let mut y = r;
        let mut d = 1 - r;
        while x <= y {
            for &(dx, dy) in &[
                (x, y),
                (y, x),
                (-x, y),
                (-y, x),
                (x, -y),
                (y, -x),
                (-x, -y),
                (-y, -x),
            ] {
                self.put_pixel(center.x + dx, center.y + dy, color);
            }
            if d < 0 {
                d += 2 * x + 3;
            } else {
                d += 2 * (x - y) + 5;
                y -= 1;
            }
            x += 1;
        }
    }

    pub fn fill_circle(&mut self, center: Point, radius: u32, color: Color) {
        let r = radius as i32;
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r {
                    self.put_pixel(center.x + x, center.y + y, color);
                }
            }
        }
    }

    pub fn draw_triangle(&mut self, p1: Point, p2: Point, p3: Point, color: Color) {
        self.draw_line(p1, p2, color);
        self.draw_line(p2, p3, color);
        self.draw_line(p3, p1, color);
    }

    pub fn fill_triangle(&mut self, p1: Point, p2: Point, p3: Point, color: Color) {
        let mut pts = [p1, p2, p3];
        pts.sort_by_key(|p| p.y);
        let (p0, p1, p2) = (pts[0], pts[1], pts[2]);
        let mut fill_span = |y: i32, x0: i32, x1: i32| {
            for x in x0..=x1 {
                self.put_pixel(x, y, color);
            }
        };
        let edge_interp = |y: i32, a: Point, b: Point| -> i32 {
            if b.y == a.y {
                a.x
            } else {
                a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y)
            }
        };
        for y in p0.y..=p2.y {
            let x_start = if y < p1.y {
                edge_interp(y, p0, p1)
            } else {
                edge_interp(y, p1, p2)
            };
            let x_end = edge_interp(y, p0, p2);
            fill_span(y, x_start, x_end);
        }
    }

    pub fn draw_rounded_rect(&mut self, r: Rect, rad: u32, color: Color) {
        self.draw_rect(r, color);
        let rad_i32 = rad as i32;

        self.draw_circle(Point::new(r.x + rad_i32, r.y + rad_i32), rad, color);
        self.draw_circle(
            Point::new(r.x + r.width as i32 - rad_i32, r.y + rad_i32),
            rad,
            color,
        );
        self.draw_circle(
            Point::new(
                r.x + r.width as i32 - rad_i32,
                r.y + r.height as i32 - rad_i32,
            ),
            rad,
            color,
        );
        self.draw_circle(
            Point::new(r.x + rad_i32, r.y + r.height as i32 - rad_i32),
            rad,
            color,
        );
    }

    pub fn draw_arc(&mut self, center: Point, radius: u32, start: f32, end: f32, color: Color) {
        for deg in (start * 100.0) as i32..=(end * 100.0) as i32 {
            let rad = (deg as f32 / 100.0).to_radians();
            let x = center.x + (rad.cos() * radius as f32) as i32;
            let y = center.y + (rad.sin() * radius as f32) as i32;
            self.put_pixel(x, y, color);
        }
    }

    pub fn fill_gradient_h(&mut self, r: Rect, from: Color, to: Color) {
        for x in 0..r.width {
            let t = x as f32 / (r.width - 1) as f32;
            let col = from.lerp(to, t);
            for y in 0..r.height {
                self.put_pixel(r.x + x as i32, r.y + y as i32, col);
            }
        }
    }

    pub fn fill_gradient_v(&mut self, r: Rect, from: Color, to: Color) {
        for y in 0..r.height {
            let t = y as f32 / (r.height - 1) as f32;
            let col = from.lerp(to, t);
            for x in 0..r.width {
                self.put_pixel(r.x + x as i32, r.y + y as i32, col);
            }
        }
    }

    pub fn draw_bmp(&mut self, img: &bmp::BmpImage, x: i32, y: i32) {
        for j in 0..img.height {
            for i in 0..img.width {
                let idx = (j * img.width + i) * 4;
                let color = Color {
                    b: img.data[idx as usize],
                    g: img.data[idx as usize + 1],
                    r: img.data[idx as usize + 2],
                    a: img.data[idx as usize + 3],
                };
                self.put_pixel(x + i as i32, y + j as i32, color);
            }
        }
    }

    pub fn draw_char(&mut self, font: &bdf::Font, ch: char, x: i32, y: i32, color: Color) {
        font.draw_char(ch, x, y, |px, py| self.put_pixel(px, py, color));
    }

    pub fn draw_text(&mut self, font: &bdf::Font, text: &str, x: i32, y: i32, color: Color) {
        font.draw_text(text, x, y, |px, py| self.put_pixel(px, py, color));
    }

    pub fn present(&mut self) -> Result<(), &'static str> {
        if self.dirty {
            self.window
                .update_with_buffer(&self.buffer, self.width as usize, self.height as usize)
                .map_err(|_| "Failed to update window")?;
            self.dirty = false;
        }
        Ok(())
    }
}
