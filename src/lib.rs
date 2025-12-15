#![allow(dead_code)]
use libm::{cosf, sinf, sqrtf};
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

    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
    pub const ORANGE: Color = Color::rgb(255, 165, 0);
    pub const PURPLE: Color = Color::rgb(128, 0, 128);
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    pub const LIGHT_GRAY: Color = Color::rgb(192, 192, 192);
    pub const DARK_GRAY: Color = Color::rgb(64, 64, 64);
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    pub fn to_u32(self) -> u32 {
        (self.b as u32) | ((self.g as u32) << 8) | ((self.r as u32) << 16) | ((self.a as u32) << 24)
    }

    pub fn blend(self, bg: Color) -> Color {
        if self.a == 255 {
            return self;
        }
        if self.a == 0 {
            return bg;
        }
        let alpha = self.a as u32;
        let inv_alpha = 255 - alpha;
        Color {
            r: ((self.r as u32 * alpha + bg.r as u32 * inv_alpha) / 255) as u8,
            g: ((self.g as u32 * alpha + bg.g as u32 * inv_alpha) / 255) as u8,
            b: ((self.b as u32 * alpha + bg.b as u32 * inv_alpha) / 255) as u8,
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
    pub fn distance_to(&self, other: Point) -> f32 {
        let dx = (other.x - self.x) as f32;
        let dy = (other.y - self.y) as f32;
        sqrtf(dx * dx + dy * dy)
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
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x
            && p.x < self.x + self.width as i32
            && p.y >= self.y
            && p.y < self.y + self.height as i32
    }
}

pub struct Sight {
    pub fb: Vec<u32>,
    pub window: Window,
    pub width: u32,
    pub height: u32,
    pub dirty: bool,
}

impl Sight {
    pub fn new(width: u32, height: u32, name: &str) -> Result<Self, &'static str> {
        let window = Window::new(
            name,
            width as usize,
            height as usize,
            WindowOptions {
                scale: Scale::X2,
                scale_mode: ScaleMode::AspectRatioStretch,
                ..WindowOptions::default()
            },
        )
        .map_err(|_| "Failed to create window {}")?;
        let fb = vec![0u32; (width * height) as usize];
        Ok(Self {
            fb,
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
        let pixel = color.to_u32();
        for v in self.fb.iter_mut() {
            *v = pixel;
        }
        self.dirty = true;
    }

    pub fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        self.fb[idx] = color.to_u32();
        self.dirty = true;
    }

    fn put_pixel_aa(&mut self, x: i32, y: i32, color: Color, _alpha: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        let existing = Color::rgba(
            ((self.fb[idx] >> 16) & 0xFF) as u8,
            ((self.fb[idx] >> 8) & 0xFF) as u8,
            (self.fb[idx] & 0xFF) as u8,
            255,
        );
        let blended = color.blend(existing);
        self.fb[idx] = blended.to_u32();
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
        let mut intery = y0 + gradient;
        for x in x0 as i32..=x1 as i32 {
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

    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.draw_line(
            Point::new(rect.x, rect.y),
            Point::new(rect.x + rect.width as i32 - 1, rect.y),
            color,
        );
        self.draw_line(
            Point::new(rect.x + rect.width as i32 - 1, rect.y),
            Point::new(
                rect.x + rect.width as i32 - 1,
                rect.y + rect.height as i32 - 1,
            ),
            color,
        );
        self.draw_line(
            Point::new(
                rect.x + rect.width as i32 - 1,
                rect.y + rect.height as i32 - 1,
            ),
            Point::new(rect.x, rect.y + rect.height as i32 - 1),
            color,
        );
        self.draw_line(
            Point::new(rect.x, rect.y + rect.height as i32 - 1),
            Point::new(rect.x, rect.y),
            color,
        );
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        for y in rect.y..(rect.y + rect.height as i32) {
            for x in rect.x..(rect.x + rect.width as i32) {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color) {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;
        while x >= y {
            let points = [
                (cx + x, cy + y),
                (cx + y, cy + x),
                (cx - y, cy + x),
                (cx - x, cy + y),
                (cx - x, cy - y),
                (cx - y, cy - x),
                (cx + y, cy - x),
                (cx + x, cy - y),
            ];
            for (px, py) in points.iter() {
                self.put_pixel(*px, *py, color);
            }
            y += 1;
            if err <= 0 {
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Color) {
        for y in -radius..=radius {
            for x in -radius..=radius {
                if x * x + y * y <= radius * radius {
                    self.put_pixel(cx + x, cy + y, color);
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
        let [p0, p1, p2] = pts;
        for y in p0.y..=p2.y {
            let mut xs = vec![];
            for (a, b) in &[(p0, p1), (p1, p2), (p0, p2)] {
                if y >= a.y && y <= b.y {
                    let t = (y - a.y) as f32 / (b.y - a.y) as f32;
                    xs.push(a.x + ((b.x - a.x) as f32 * t) as i32);
                }
            }
            xs.sort();
            for x in xs[0]..=xs[1] {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn draw_rounded_rect(&mut self, rect: Rect, radius: i32, color: Color) {
        self.draw_rect(rect, color);
        self.draw_circle(rect.x + radius, rect.y + radius, radius, color);
        self.draw_circle(
            rect.x + rect.width as i32 - radius - 1,
            rect.y + radius,
            radius,
            color,
        );
        self.draw_circle(
            rect.x + radius,
            rect.y + rect.height as i32 - radius - 1,
            radius,
            color,
        );
        self.draw_circle(
            rect.x + rect.width as i32 - radius - 1,
            rect.y + rect.height as i32 - radius - 1,
            radius,
            color,
        );
    }

    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: i32, color: Color) {
        self.fill_rect(
            Rect::new(
                rect.x + radius,
                rect.y,
                rect.width - 2 * radius as u32,
                rect.height,
            ),
            color,
        );
        self.fill_rect(
            Rect::new(
                rect.x,
                rect.y + radius,
                radius as u32,
                rect.height - 2 * radius as u32,
            ),
            color,
        );
        self.fill_rect(
            Rect::new(
                rect.x + rect.width as i32 - radius,
                rect.y + radius,
                radius as u32,
                rect.height - 2 * radius as u32,
            ),
            color,
        );
        self.fill_circle(rect.x + radius, rect.y + radius, radius, color);
        self.fill_circle(
            rect.x + rect.width as i32 - radius - 1,
            rect.y + radius,
            radius,
            color,
        );
        self.fill_circle(
            rect.x + radius,
            rect.y + rect.height as i32 - radius - 1,
            radius,
            color,
        );
        self.fill_circle(
            rect.x + rect.width as i32 - radius - 1,
            rect.y + rect.height as i32 - radius - 1,
            radius,
            color,
        );
    }

    pub fn fill_gradient_h(&mut self, rect: Rect, c1: Color, c2: Color) {
        for x in 0..rect.width as i32 {
            let t = x as f32 / rect.width as f32;
            let col = c1.lerp(c2, t);
            for y in 0..rect.height as i32 {
                self.put_pixel(rect.x + x, rect.y + y, col);
            }
        }
    }

    pub fn fill_gradient_v(&mut self, rect: Rect, c1: Color, c2: Color) {
        for y in 0..rect.height as i32 {
            let t = y as f32 / rect.height as f32;
            let col = c1.lerp(c2, t);
            for x in 0..rect.width as i32 {
                self.put_pixel(rect.x + x, rect.y + y, col);
            }
        }
    }

    pub fn draw_bmp(&mut self, bmp: bmp::BmpImage, x: i32, y: i32) {
        for row in 0..bmp.height as i32 {
            for col in 0..bmp.width as i32 {
                let idx = ((row * bmp.width as i32 + col) * 4) as usize;
                let color = Color::rgba(
                    bmp.data[idx],
                    bmp.data[idx + 1],
                    bmp.data[idx + 2],
                    bmp.data[idx + 3],
                );
                self.put_pixel(x + col, y + row, color);
            }
        }
    }

    pub fn draw_arc(&mut self, cx: i32, cy: i32, radius: i32, start: f32, end: f32, color: Color) {
        let steps = (radius * 4) as usize;
        for i in 0..=steps {
            let t = start + (end - start) * i as f32 / steps as f32;
            let px = cx + (radius as f32 * cosf(t)) as i32;
            let py = cy + (radius as f32 * sinf(t)) as i32;
            self.put_pixel(px, py, color);
        }
    }

    pub fn present(&mut self) -> Result<(), &'static str> {
        self.window
            .update_with_buffer(&self.fb, self.width as usize, self.height as usize)
            .map_err(|_| "Failed to update window")?;
        self.dirty = false;
        Ok(())
    }

    pub fn force_present(&mut self) -> Result<(), &'static str> {
        self.present()
    }
}

trait FloatExt {
    fn fract(self) -> Self;
    fn floor(self) -> Self;
}
impl FloatExt for f32 {
    fn fract(self) -> f32 {
        self - self.floor()
    }
    fn floor(self) -> f32 {
        libm::floorf(self)
    }
}
