use anyhow::Result;

const PALETTE: &[u8] = b" .'`^\",:;Il!i><~+_-?][}{1)(|\\tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$";

#[derive(Clone, Debug)]
pub struct AsciiFrame {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<(char, u8, u8, u8)>,
}

impl AsciiFrame {
    #[allow(dead_code)]
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![(' ', 0, 0, 0); (width * height) as usize],
        }
    }

    pub fn from_rgb_data(data: &[u8], width: u16, height: u16, mono: bool) -> Result<Self> {
        let mut cells = Vec::with_capacity((width * height) as usize);
        
        for y in 0..height {
            for x in 0..width {
                let i = ((y * width + x) * 3) as usize;
                if i + 2 >= data.len() {
                    cells.push((' ', 0, 0, 0));
                    continue;
                }
                
                let (r, g, b) = (data[i], data[i + 1], data[i + 2]);
                let ch = ascii_for_rgb(r, g, b);
                
                if mono {
                    let lum = luminance(r, g, b);
                    cells.push((ch, lum, lum, lum));
                } else {
                    cells.push((ch, r, g, b));
                }
            }
        }
        
        Ok(Self { width, height, cells })
    }

    #[allow(dead_code)]
    pub fn to_string_colored(&self) -> String {
        let mut result = String::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let (ch, r, g, b) = self.cells[idx];
                result.push_str(&format!("\x1b[38;2;{};{};{}m{}", r, g, b, ch));
            }
            result.push_str("\x1b[0m\n");
        }
        result
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.width.to_le_bytes());
        data.extend_from_slice(&self.height.to_le_bytes());
        
        for &(ch, r, g, b) in &self.cells {
            data.push(ch as u8);
            data.push(r);
            data.push(g);
            data.push(b);
        }
        data
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            anyhow::bail!("Invalid frame data");
        }
        
        let width = u16::from_le_bytes([data[0], data[1]]);
        let height = u16::from_le_bytes([data[2], data[3]]);
        let expected_len = 4 + (width * height * 4) as usize;
        
        if data.len() != expected_len {
            anyhow::bail!("Invalid frame data length");
        }
        
        let mut cells = Vec::new();
        let mut i = 4;
        
        while i < data.len() {
            let ch = data[i] as char;
            let r = data[i + 1];
            let g = data[i + 2];
            let b = data[i + 3];
            cells.push((ch, r, g, b));
            i += 4;
        }
        
        Ok(Self { width, height, cells })
    }
}

fn luminance(r: u8, g: u8, b: u8) -> u8 {
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8
}

fn ascii_for_rgb(r: u8, g: u8, b: u8) -> char {
    let lum = luminance(r, g, b) as usize;
    let idx = (lum * (PALETTE.len() - 1)) / 255;
    PALETTE[idx] as char
}