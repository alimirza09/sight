#![no_std]
use libm::{cosf, sinf, sqrtf};
extern crate alloc;
pub mod bdf;
pub mod bmp;

const SYS_GPU_INFO: u64 = 12;
const SYS_GPU_MAP: u64 = 13;
const SYS_GPU_FLUSH: u64 = 14;

#[inline(always)]
fn syscall3(num: u64, a0: u64, a1: u64, a2: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") num,
            in("rdi") a0,
            in("rsi") a1,
            in("rdx") a2,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

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
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
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

pub struct Sight {
    fb: *mut u32,
    width: u32,
    height: u32,
    dirty: bool,
}

impl Sight {
    pub fn new() -> Result<Self, &'static str> {
        let info = syscall3(SYS_GPU_INFO, 0, 0, 0);
        if info == u64::MAX {
            return Err("GPU not available");
        }

        let width = (info & 0xFFFFFFFF) as u32;
        let height = (info >> 32) as u32;

        const FB_ADDR: u64 = 0x8000_0000;
        let mapped = syscall3(SYS_GPU_MAP, FB_ADDR, 0, 0);
        if mapped == u64::MAX {
            return Err("Failed to map framebuffer");
        }

        Ok(Self {
            fb: mapped as *mut u32,
            width,
            height,
            dirty: false,
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
        unsafe {
            for y in 0..self.height {
                for x in 0..self.width {
                    let offset = (y * self.width + x) as usize;
                    *self.fb.add(offset) = pixel;
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
            let offset = (y as u32 * self.width + x as u32) as usize;
            *self.fb.add(offset) = color.to_u32();
        }
        self.dirty = true;
    }

    fn put_pixel_aa(&mut self, x: i32, y: i32, color: Color, alpha: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }

        let blended_color = Color::rgba(
            color.r,
            color.g,
            color.b,
            (color.a as f32 * alpha.clamp(0.0, 1.0)) as u8,
        );

        let final_color = blended_color.blend(Color::BLACK);

        unsafe {
            let offset = (y as u32 * self.width + x as u32) as usize;
            *self.fb.add(offset) = final_color.to_u32();
        }
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
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.draw_circle_points(center, x, y, color);

            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }

            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    fn draw_circle_points(&mut self, center: Point, x: i32, y: i32, color: Color) {
        self.put_pixel(center.x + x, center.y + y, color);
        self.put_pixel(center.x + y, center.y + x, color);
        self.put_pixel(center.x - y, center.y + x, color);
        self.put_pixel(center.x - x, center.y + y, color);
        self.put_pixel(center.x - x, center.y - y, color);
        self.put_pixel(center.x - y, center.y - x, color);
        self.put_pixel(center.x + y, center.y - x, color);
        self.put_pixel(center.x + x, center.y - y, color);
    }

    pub fn fill_circle(&mut self, center: Point, radius: i32, color: Color) {
        let r = radius as f32;
        let r_outer = r + 1.0;
        let r_inner = r - 1.0;

        let min_x = (center.x - radius - 1).max(0);
        let max_x = (center.x + radius + 1).min(self.width as i32 - 1);
        let min_y = (center.y - radius - 1).max(0);
        let max_y = (center.y + radius + 1).min(self.height as i32 - 1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = (x - center.x) as f32;
                let dy = (y - center.y) as f32;
                let dist = sqrtf(dx * dx + dy * dy);

                if dist <= r_inner {
                    self.put_pixel(x, y, color);
                } else if dist < r_outer {
                    let alpha = r_outer - dist;
                    self.put_pixel_aa(x, y, color, alpha);
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

    pub fn draw_rounded_rect(&mut self, rect: Rect, radius: i32, color: Color) {
        if radius <= 0 {
            self.draw_rect(rect, color);
            return;
        }

        let x = rect.x;
        let y = rect.y;
        let w = rect.width as i32;
        let h = rect.height as i32;
        let r = radius.min(w / 2).min(h / 2);

        self.draw_line(Point::new(x + r, y), Point::new(x + w - r, y), color);
        self.draw_line(
            Point::new(x + w, y + r),
            Point::new(x + w, y + h - r),
            color,
        );
        self.draw_line(
            Point::new(x + w - r, y + h),
            Point::new(x + r, y + h),
            color,
        );
        self.draw_line(Point::new(x, y + h - r), Point::new(x, y + r), color);

        let pi = 3.14159265359;
        self.draw_arc(Point::new(x + r, y + r), r, pi, pi * 1.5, color);
        self.draw_arc(Point::new(x + w - r, y + r), r, pi * 1.5, pi * 2.0, color);
        self.draw_arc(Point::new(x + w - r, y + h - r), r, 0.0, pi * 0.5, color);
        self.draw_arc(Point::new(x + r, y + h - r), r, pi * 0.5, pi, color);
    }

    pub fn draw_arc(
        &mut self,
        center: Point,
        radius: i32,
        start_angle: f32,
        end_angle: f32,
        color: Color,
    ) {
        let r = radius as f32;
        let pi2 = 6.28318530718;

        let start = start_angle;
        let mut end = end_angle;

        while end < start {
            end += pi2;
        }

        let angle_range = end - start;
        let circumference = pi2 * r;
        let arc_length = (angle_range / pi2) * circumference;
        let steps = (arc_length * 2.0) as i32;
        let steps = steps.max(30).min(1000);

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let angle = start + angle_range * t;
            let x = center.x + (cosf(angle) * r) as i32;
            let y = center.y + (sinf(angle) * r) as i32;
            self.put_pixel(x, y, color);
        }
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

    pub fn draw_bmp(&mut self, bmp: &bmp::BmpImage, x: i32, y: i32) {
        for iy in 0..bmp.height {
            for ix in 0..bmp.width {
                let pixel_idx = ((iy * bmp.width + ix) * 4) as usize;
                if pixel_idx + 3 < bmp.data.len() {
                    let color = Color::rgba(
                        bmp.data[pixel_idx + 2],
                        bmp.data[pixel_idx + 1],
                        bmp.data[pixel_idx],
                        bmp.data[pixel_idx + 3],
                    );
                    if color.a > 0 {
                        self.put_pixel(x + ix as i32, y + iy as i32, color);
                    }
                }
            }
        }
    }

    pub fn present(&mut self) -> Result<(), &'static str> {
        if !self.dirty {
            return Ok(());
        }

        let result = syscall3(SYS_GPU_FLUSH, 0, 0, 0);
        if result == u64::MAX {
            return Err("Failed to flush GPU");
        }

        self.dirty = false;
        Ok(())
    }

    pub fn force_present(&mut self) -> Result<(), &'static str> {
        let result = syscall3(SYS_GPU_FLUSH, 0, 0, 0);
        if result == u64::MAX {
            return Err("Failed to flush GPU");
        }

        self.dirty = false;
        Ok(())
    }
}

trait FloatExt {
    fn fract(self) -> Self;
    fn floor(self) -> Self;
    fn round(self) -> Self;
}

impl FloatExt for f32 {
    fn fract(self) -> f32 {
        self - self.floor()
    }

    fn floor(self) -> f32 {
        libm::floorf(self)
    }

    fn round(self) -> f32 {
        libm::roundf(self)
    }
}
