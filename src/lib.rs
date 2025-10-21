#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn to_u32(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
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

pub trait Framebuffer {
    fn dimensions(&self) -> (u32, u32);

    unsafe fn write_pixel(&mut self, x: u32, y: u32, color: u32) -> bool;

    fn flush(&mut self) -> Result<(), &'static str>;
}

pub struct Sight<F: Framebuffer> {
    framebuffer: F,
    width: u32,
    height: u32,
    dirty: bool,
}

impl<F: Framebuffer> Sight<F> {
    pub fn new(framebuffer: F) -> Self {
        let (width, height) = framebuffer.dimensions();
        Self {
            framebuffer,
            width,
            height,
            dirty: false,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn clear(&mut self, color: Color) {
        let pixel = color.to_u32();
        unsafe {
            for y in 0..self.height {
                for x in 0..self.width {
                    self.framebuffer.write_pixel(x, y, pixel);
                }
            }
        }
        self.dirty = true;
    }

    pub fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }

        unsafe {
            if self
                .framebuffer
                .write_pixel(x as u32, y as u32, color.to_u32())
            {
                self.dirty = true;
            }
        }
    }

    pub fn draw_line(&mut self, p1: Point, p2: Point, color: Color) {
        let mut x0 = p1.x;
        let mut y0 = p1.y;
        let x1 = p2.x;
        let y1 = p2.y;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.put_pixel(x0, y0, color);

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * error;
            if e2 >= dy {
                if x0 == x1 {
                    break;
                }
                error += dy;
                x0 += sx;
            }
            if e2 <= dx {
                if y0 == y1 {
                    break;
                }
                error += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_thick_line(&mut self, p1: Point, p2: Point, color: Color, thickness: u32) {
        if thickness <= 1 {
            self.draw_line(p1, p2, color);
            return;
        }

        let dx = (p2.x - p1.x) as f32;
        let dy = (p2.y - p1.y) as f32;
        let length = (dx * dx + dy * dy).sqrt();

        if length == 0.0 {
            return;
        }

        let nx = -dy / length;
        let ny = dx / length;
        let half_thickness = thickness as f32 / 2.0;

        for i in 0..thickness {
            let offset = (i as f32 - half_thickness + 0.5) as i32;
            let offset_x = (nx * offset as f32) as i32;
            let offset_y = (ny * offset as f32) as i32;

            self.draw_line(
                Point::new(p1.x + offset_x, p1.y + offset_y),
                Point::new(p2.x + offset_x, p2.y + offset_y),
                color,
            );
        }
    }

    pub fn present(&mut self) -> Result<(), &'static str> {
        if !self.dirty {
            return Ok(());
        }

        self.framebuffer.flush()?;
        self.dirty = false;
        Ok(())
    }

    pub fn force_present(&mut self) -> Result<(), &'static str> {
        self.framebuffer.flush()?;
        self.dirty = false;
        Ok(())
    }
}
