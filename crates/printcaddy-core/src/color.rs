/// sRGB colour stored as three bytes. New bodies use [`DEFAULT_MODEL_COLOR`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Slate grey used for every new body in the current PrintCaddy app.
pub const DEFAULT_MODEL_COLOR: Color = Color {
    r: 0x8A,
    g: 0x98,
    b: 0xA6,
};

impl Color {
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xff) as u8,
            g: ((hex >> 8) & 0xff) as u8,
            b: (hex & 0xff) as u8,
        }
    }

    pub fn to_hex(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    pub fn hex_string(self) -> String {
        format!("#{:06X}", self.to_hex())
    }

    pub fn to_rgb_f32(self) -> [f32; 3] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        ]
    }
}
