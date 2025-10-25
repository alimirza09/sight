extern crate alloc;
use alloc::vec::Vec;

pub struct BmpImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl BmpImage {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 54 {
            return Err("File too small");
        }

        if bytes[0] != b'B' || bytes[1] != b'M' {
            return Err("Not a BMP file");
        }

        let dib_header_size = read_u32_le(bytes, 14)?;
        if dib_header_size < 40 {
            return Err("Unsupported BMP format");
        }

        let width = read_i32_le(bytes, 18)? as u32;
        let height_signed = read_i32_le(bytes, 22)?;
        let height = height_signed.unsigned_abs();
        let top_down = height_signed < 0;

        if width == 0 || height == 0 {
            return Err("Invalid dimensions");
        }

        let planes = read_u16_le(bytes, 26)?;
        let bits_per_pixel = read_u16_le(bytes, 28)?;
        let compression = read_u32_le(bytes, 30)?;

        if planes != 1 {
            return Err("Invalid planes");
        }
        if compression != 0 && compression != 3 {
            return Err("Compressed BMP not supported");
        }

        let data_offset = read_u32_le(bytes, 10)? as usize;
        if data_offset >= bytes.len() {
            return Err("Invalid data offset");
        }

        let pixel_data = &bytes[data_offset..];

        let bgra_data = match bits_per_pixel {
            24 => parse_24bit(pixel_data, width, height, top_down)?,
            32 => parse_32bit(pixel_data, width, height, top_down)?,
            _ => return Err("Only 24 or 32-bit BMP supported"),
        };

        Ok(BmpImage {
            width,
            height,
            data: bgra_data,
        })
    }
}

#[inline]
fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, &'static str> {
    bytes
        .get(offset..offset + 2)
        .and_then(|s| s.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or("Out of bounds read")
}

#[inline]
fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, &'static str> {
    bytes
        .get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or("Out of bounds read")
}

#[inline]
fn read_i32_le(bytes: &[u8], offset: usize) -> Result<i32, &'static str> {
    bytes
        .get(offset..offset + 4)
        .and_then(|s| s.try_into().ok())
        .map(i32::from_le_bytes)
        .ok_or("Out of bounds read")
}

fn parse_24bit(
    pixel_data: &[u8],
    width: u32,
    height: u32,
    top_down: bool,
) -> Result<Vec<u8>, &'static str> {
    let row_size = (width
        .checked_mul(3)
        .and_then(|w| w.checked_add(3))
        .ok_or("Row size overflow")?
        / 4)
        * 4;

    let required_size = row_size.checked_mul(height).ok_or("Data size overflow")? as usize;

    if pixel_data.len() < required_size {
        return Err("Insufficient pixel data");
    }

    let mut bgra_data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        let actual_y = if top_down { y } else { height - 1 - y };
        let row_offset = (actual_y * row_size) as usize;

        for x in 0..width {
            let pixel_offset = row_offset + (x * 3) as usize;

            bgra_data.push(pixel_data[pixel_offset]);
            bgra_data.push(pixel_data[pixel_offset + 1]);
            bgra_data.push(pixel_data[pixel_offset + 2]);
            bgra_data.push(255);
        }
    }

    Ok(bgra_data)
}

fn parse_32bit(
    pixel_data: &[u8],
    width: u32,
    height: u32,
    top_down: bool,
) -> Result<Vec<u8>, &'static str> {
    let row_size = width.checked_mul(4).ok_or("Row size overflow")?;

    let required_size = row_size.checked_mul(height).ok_or("Data size overflow")? as usize;

    if pixel_data.len() < required_size {
        return Err("Insufficient pixel data");
    }

    let mut bgra_data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        let actual_y = if top_down { y } else { height - 1 - y };
        let row_offset = (actual_y * row_size) as usize;

        for x in 0..width {
            let pixel_offset = row_offset + (x * 4) as usize;

            bgra_data.push(pixel_data[pixel_offset]);
            bgra_data.push(pixel_data[pixel_offset + 1]);
            bgra_data.push(pixel_data[pixel_offset + 2]);
            bgra_data.push(pixel_data[pixel_offset + 3]);
        }
    }

    Ok(bgra_data)
}
