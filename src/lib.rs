#![no_std]
use libm::{cosf, sinf, sqrtf};

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

    pub fn blend(self, background: Color) -> Color {
        if self.a == 255 {
            return self;
        }
        if self.a == 0 {
            return background;
        }

        let alpha = self.a as u32;
        let inv_alpha = 255 - alpha;

        Color {
            r: ((self.r as u32 * alpha + background.r as u32 * inv_alpha) / 255) as u8,
            g: ((self.g as u32 * alpha + background.g as u32 * inv_alpha) / 255) as u8,
            b: ((self.b as u32 * alpha + background.b as u32 * inv_alpha) / 255) as u8,
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

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width as i32
            && point.y >= self.y
            && point.y < self.y + self.height as i32
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width as i32
            && self.x + self.width as i32 > other.x
            && self.y < other.y + other.height as i32
            && self.y + self.height as i32 > other.y
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
        let length = sqrtf(dx * dx + dy * dy);

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

    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        let x2 = rect.x + rect.width as i32 - 1;
        let y2 = rect.y + rect.height as i32 - 1;

        self.draw_line(Point::new(rect.x, rect.y), Point::new(x2, rect.y), color);
        self.draw_line(Point::new(x2, rect.y), Point::new(x2, y2), color);
        self.draw_line(Point::new(x2, y2), Point::new(rect.x, y2), color);
        self.draw_line(Point::new(rect.x, y2), Point::new(rect.x, rect.y), color);
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let x1 = rect.x.max(0);
        let y1 = rect.y.max(0);
        let x2 = (rect.x + rect.width as i32).min(self.width as i32);
        let y2 = (rect.y + rect.height as i32).min(self.height as i32);

        for y in y1..y2 {
            for x in x1..x2 {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn draw_circle(&mut self, center: Point, radius: i32, color: Color) {
        let mut x = 0;
        let mut y = radius;
        let mut d = 3 - 2 * radius;

        while x <= y {
            self.put_pixel(center.x + x, center.y + y, color);
            self.put_pixel(center.x - x, center.y + y, color);
            self.put_pixel(center.x + x, center.y - y, color);
            self.put_pixel(center.x - x, center.y - y, color);
            self.put_pixel(center.x + y, center.y + x, color);
            self.put_pixel(center.x - y, center.y + x, color);
            self.put_pixel(center.x + y, center.y - x, color);
            self.put_pixel(center.x - y, center.y - x, color);

            if d < 0 {
                d += 4 * x + 6;
            } else {
                d += 4 * (x - y) + 10;
                y -= 1;
            }
            x += 1;
        }
    }

    pub fn fill_circle(&mut self, center: Point, radius: i32, color: Color) {
        let r_squared = radius * radius;

        for y in -radius..=radius {
            for x in -radius..=radius {
                if x * x + y * y <= r_squared {
                    self.put_pixel(center.x + x, center.y + y, color);
                }
            }
        }
    }

    pub fn draw_ellipse(&mut self, center: Point, rx: i32, ry: i32, color: Color) {
        let mut x = 0;
        let mut y = ry;
        let rx2 = rx * rx;
        let ry2 = ry * ry;
        let mut p = ry2 - (rx2 * ry) + (rx2 / 4);

        while (ry2 * x) <= (rx2 * y) {
            self.put_pixel(center.x + x, center.y + y, color);
            self.put_pixel(center.x - x, center.y + y, color);
            self.put_pixel(center.x + x, center.y - y, color);
            self.put_pixel(center.x - x, center.y - y, color);

            if p < 0 {
                x += 1;
                p += ry2 * (2 * x + 1);
            } else {
                x += 1;
                y -= 1;
                p += ry2 * (2 * x + 1) - rx2 * (2 * y);
            }
        }

        p = ry2 * (x * x + x) + rx2 * (y - 1) * (y - 1) - rx2 * ry2;

        while y >= 0 {
            self.put_pixel(center.x + x, center.y + y, color);
            self.put_pixel(center.x - x, center.y + y, color);
            self.put_pixel(center.x + x, center.y - y, color);
            self.put_pixel(center.x - x, center.y - y, color);

            if p > 0 {
                y -= 1;
                p -= rx2 * (2 * y + 1);
            } else {
                x += 1;
                y -= 1;
                p += ry2 * (2 * x + 1) - rx2 * (2 * y + 1);
            }
        }
    }

    pub fn draw_triangle(&mut self, p1: Point, p2: Point, p3: Point, color: Color) {
        self.draw_line(p1, p2, color);
        self.draw_line(p2, p3, color);
        self.draw_line(p3, p1, color);
    }

    pub fn fill_triangle(&mut self, p1: Point, p2: Point, p3: Point, color: Color) {
        let mut points = [p1, p2, p3];
        if points[0].y > points[1].y {
            points.swap(0, 1);
        }
        if points[1].y > points[2].y {
            points.swap(1, 2);
        }
        if points[0].y > points[1].y {
            points.swap(0, 1);
        }

        let [top, mid, bottom] = points;

        if mid.y == bottom.y {
            self.fill_flat_bottom_triangle(top, mid, bottom, color);
        } else if top.y == mid.y {
            self.fill_flat_top_triangle(top, mid, bottom, color);
        } else {
            let split_x = top.x
                + ((mid.y - top.y) as f32 / (bottom.y - top.y) as f32 * (bottom.x - top.x) as f32)
                    as i32;
            let split = Point::new(split_x, mid.y);

            self.fill_flat_bottom_triangle(top, mid, split, color);
            self.fill_flat_top_triangle(mid, split, bottom, color);
        }
    }

    fn fill_flat_bottom_triangle(&mut self, top: Point, left: Point, right: Point, color: Color) {
        let (left, right) = if left.x > right.x {
            (right, left)
        } else {
            (left, right)
        };

        let dy = (left.y - top.y) as f32;
        if dy == 0.0 {
            return;
        }

        let slope_left = (left.x - top.x) as f32 / dy;
        let slope_right = (right.x - top.x) as f32 / dy;

        let mut x_left = top.x as f32;
        let mut x_right = top.x as f32;

        for y in top.y..=left.y {
            for x in (x_left as i32)..=(x_right as i32) {
                self.put_pixel(x, y, color);
            }
            x_left += slope_left;
            x_right += slope_right;
        }
    }

    fn fill_flat_top_triangle(&mut self, left: Point, right: Point, bottom: Point, color: Color) {
        let (left, right) = if left.x > right.x {
            (right, left)
        } else {
            (left, right)
        };

        let dy = (bottom.y - left.y) as f32;
        if dy == 0.0 {
            return;
        }

        let slope_left = (bottom.x - left.x) as f32 / dy;
        let slope_right = (bottom.x - right.x) as f32 / dy;

        let mut x_left = left.x as f32;
        let mut x_right = right.x as f32;

        for y in left.y..=bottom.y {
            for x in (x_left as i32)..=(x_right as i32) {
                self.put_pixel(x, y, color);
            }
            x_left += slope_left;
            x_right += slope_right;
        }
    }

    pub fn draw_polygon(&mut self, points: &[Point], color: Color) {
        if points.len() < 2 {
            return;
        }

        for i in 0..points.len() {
            let next = (i + 1) % points.len();
            self.draw_line(points[i], points[next], color);
        }
    }

    pub fn draw_arc(
        &mut self,
        center: Point,
        radius: i32,
        start_angle: f32,
        end_angle: f32,
        color: Color,
    ) {
        let steps = (radius as f32 * (end_angle - start_angle).abs() / 2.0) as i32;
        let steps = steps.max(10);

        for i in 0..steps {
            let angle = start_angle + (end_angle - start_angle) * (i as f32 / steps as f32);
            let x = center.x + (cosf(angle) * radius as f32) as i32;
            let y = center.y + (sinf(angle) * radius as f32) as i32;
            self.put_pixel(x, y, color);
        }
    }

    pub fn draw_rounded_rect(&mut self, rect: Rect, radius: i32, color: Color) {
        let x2 = rect.x + rect.width as i32 - 1;
        let y2 = rect.y + rect.height as i32 - 1;

        self.draw_arc(
            Point::new(rect.x + radius, rect.y + radius),
            radius,
            3.14159,
            4.71239,
            color,
        );
        self.draw_arc(
            Point::new(x2 - radius, rect.y + radius),
            radius,
            4.71239,
            6.28318,
            color,
        );
        self.draw_arc(
            Point::new(x2 - radius, y2 - radius),
            radius,
            0.0,
            1.5708,
            color,
        );
        self.draw_arc(
            Point::new(rect.x + radius, y2 - radius),
            radius,
            1.5708,
            3.14159,
            color,
        );

        self.draw_line(
            Point::new(rect.x + radius, rect.y),
            Point::new(x2 - radius, rect.y),
            color,
        );
        self.draw_line(
            Point::new(x2, rect.y + radius),
            Point::new(x2, y2 - radius),
            color,
        );
        self.draw_line(
            Point::new(x2 - radius, y2),
            Point::new(rect.x + radius, y2),
            color,
        );
        self.draw_line(
            Point::new(rect.x, y2 - radius),
            Point::new(rect.x, rect.y + radius),
            color,
        );
    }

    pub fn fill_gradient_h(&mut self, rect: Rect, start_color: Color, end_color: Color) {
        let x1 = rect.x.max(0);
        let x2 = (rect.x + rect.width as i32).min(self.width as i32);
        let y1 = rect.y.max(0);
        let y2 = (rect.y + rect.height as i32).min(self.height as i32);

        for x in x1..x2 {
            let t = (x - x1) as f32 / (x2 - x1) as f32;
            let color = start_color.lerp(end_color, t);
            for y in y1..y2 {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn fill_gradient_v(&mut self, rect: Rect, start_color: Color, end_color: Color) {
        let x1 = rect.x.max(0);
        let x2 = (rect.x + rect.width as i32).min(self.width as i32);
        let y1 = rect.y.max(0);
        let y2 = (rect.y + rect.height as i32).min(self.height as i32);

        for y in y1..y2 {
            let t = (y - y1) as f32 / (y2 - y1) as f32;
            let color = start_color.lerp(end_color, t);
            for x in x1..x2 {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn draw_bezier_quad(&mut self, p0: Point, p1: Point, p2: Point, color: Color) {
        let steps = 50;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            let t_inv = 1.0 - t;

            let x = (t_inv * t_inv * p0.x as f32
                + 2.0 * t_inv * t * p1.x as f32
                + t * t * p2.x as f32) as i32;
            let y = (t_inv * t_inv * p0.y as f32
                + 2.0 * t_inv * t * p1.y as f32
                + t * t * p2.y as f32) as i32;

            self.put_pixel(x, y, color);
        }
    }

    pub fn draw_bezier_cubic(&mut self, p0: Point, p1: Point, p2: Point, p3: Point, color: Color) {
        let steps = 100;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            let t_inv = 1.0 - t;
            let t_inv2 = t_inv * t_inv;
            let t_inv3 = t_inv2 * t_inv;
            let t2 = t * t;
            let t3 = t2 * t;

            let x = (t_inv3 * p0.x as f32
                + 3.0 * t_inv2 * t * p1.x as f32
                + 3.0 * t_inv * t2 * p2.x as f32
                + t3 * p3.x as f32) as i32;
            let y = (t_inv3 * p0.y as f32
                + 3.0 * t_inv2 * t * p1.y as f32
                + 3.0 * t_inv * t2 * p2.y as f32
                + t3 * p3.y as f32) as i32;

            self.put_pixel(x, y, color);
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

trait ClampExt {
    fn clamp(self, min: Self, max: Self) -> Self;
}

impl ClampExt for f32 {
    fn clamp(self, min: f32, max: f32) -> f32 {
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}
