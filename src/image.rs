pub const IMAGE_WIDTH: usize = 320;
pub const IMAGE_HEIGHT: usize = 172;
pub const MATRIX_WIDTH: usize = 7;
pub const MATRIX_HEIGHT: usize = 7;

const HEART_BITMAP: [[u8; MATRIX_WIDTH]; MATRIX_HEIGHT] = [
    [0, 0, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, 1, 0, 0],
    [0, 1, 1, 1, 1, 1, 0],
    [0, 1, 1, 1, 1, 1, 0],
    [0, 0, 1, 1, 1, 0, 0],
    [0, 0, 0, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0],
];

const SMILEY_BITMAP: [[u8; MATRIX_WIDTH]; MATRIX_HEIGHT] = [
    [0, 0, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, 1, 0, 0],
    [0, 0, 0, 0, 0, 0, 0],
    [0, 1, 0, 0, 0, 1, 0],
    [0, 0, 1, 1, 1, 0, 0],
    [0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0],
];

const OFF_BITMAP: [[u8; MATRIX_WIDTH]; MATRIX_HEIGHT] = [[0; _]; _];

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Bitmap {
    HEART,
    SMILEY,
    OFF
}

impl Into<[[u8; MATRIX_WIDTH]; MATRIX_HEIGHT]> for Bitmap {
    fn into(self) -> [[u8; MATRIX_WIDTH]; MATRIX_HEIGHT] {
        match self {
            Self::HEART => HEART_BITMAP,
            Self::SMILEY => SMILEY_BITMAP,
            Self::OFF => OFF_BITMAP
        }
    }
}

// #[derive(Clone, Copy, Debug)]
// pub enum Hue {
//     RED,
//     GREEN,
//     PURPLE,
// }

// impl Into<u8> for Hue {
//     fn into(self) -> u8 {
//         match self {
//             Self::RED => 0,
//             Self::GREEN => 128,
//             Self::PURPLE => 191,
//         }
//     }
// }

#[derive(Clone, Copy, Debug)]
pub struct PixelRGB {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct PixelHSL {
    h: u8,
    s: u8,
    l: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum Pixel {
    RGB(PixelRGB),
    HSL(PixelHSL),
    EMPTY,
}

pub struct PixelMatrixImage {
    data: [[Pixel; MATRIX_WIDTH]; MATRIX_HEIGHT],
}

pub struct ScreenImage {
    data: [[Pixel; IMAGE_WIDTH]; IMAGE_HEIGHT],
}

impl PixelRGB {
    fn to_hsl(&self) -> PixelHSL {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);

        let delta = max - min;

        let l = (max + min) / 2.0;

        let (h, s) = if delta == 0.0 {
            (0.0, 0.0)
        } else {
            let s = delta / (1.0 - (2.0 * l - 1.0).abs());

            let h = if max == r {
                60.0 * (((g - b) / delta) % 6.0)
            } else if max == g {
                60.0 * ((b - r) / delta + 2.0)
            } else {
                60.0 * ((r - g) / delta + 4.0)
            };

            (if h < 0.0 { h + 360.0 } else { h }, s)
        };

        PixelHSL {
            h: (h / 360.0 * 255.0).round() as u8,
            s: (s * 255.0).round() as u8,
            l: (l * 255.0).round() as u8,
        }
    }

    pub(crate) fn to_buf(&self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

impl PixelHSL {
    fn to_rgb(&self) -> PixelRGB {
        let h = self.h as f32 / 255.0 * 360.0;
        let s = self.s as f32 / 255.0;
        let l = self.l as f32 / 255.0;

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r, g, b) = match h {
            h if h < 60.0 => (c, x, 0.0),
            h if h < 120.0 => (x, c, 0.0),
            h if h < 180.0 => (0.0, c, x),
            h if h < 240.0 => (0.0, x, c),
            h if h < 300.0 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        PixelRGB {
            r: ((r + m) * 255.0).round() as u8,
            g: ((g + m) * 255.0).round() as u8,
            b: ((b + m) * 255.0).round() as u8,
        }
    }

    pub(crate) fn to_buf(&self) -> [u8; 3] {
        [self.h, self.s, self.l]
    }
}

impl Pixel {
    pub fn to_rgb(&self) -> PixelRGB {
        match self {
            Self::HSL(hsl) => hsl.to_rgb(),
            Self::RGB(rgb) => *rgb,
            Self::EMPTY => PixelRGB { r: 0, g: 0, b: 0 },
        }
    }

    pub fn to_hsl(&self) -> PixelHSL {
        match self {
            Self::HSL(hsl) => *hsl,
            Self::RGB(rgb) => rgb.to_hsl(),
            Self::EMPTY => PixelHSL { h: 0, s: 0, l: 0 },
        }
    }
}

impl PixelMatrixImage {
    pub fn from_bitmap_hue(bitmap: [[u8; MATRIX_WIDTH]; MATRIX_HEIGHT], hue: u8) -> Self {
        let mut data = [[Pixel::EMPTY {}; MATRIX_WIDTH]; MATRIX_HEIGHT];

        for i in 0..MATRIX_HEIGHT {
            for j in 0..MATRIX_WIDTH {
                let d = bitmap[i][j] * 255;
                data[i][j] = Pixel::HSL(PixelHSL { h: hue, s: d, l: d })
            }
        }

        Self { data: data }
    }

    pub fn to_hsl_buf(&self) -> [u8; MATRIX_WIDTH * MATRIX_HEIGHT * 3] {
        let mut buf: [u8; _] = [0; MATRIX_WIDTH * MATRIX_HEIGHT * 3];

        for i in 0..MATRIX_HEIGHT {
            for j in 0..MATRIX_WIDTH {
                let p = self.data[i][j].to_hsl().to_buf();
                let s = 3 * ((i * MATRIX_WIDTH) + j);
                let e = s + 3;
                buf[s..e].copy_from_slice(&p);
            }
        }

        buf
    }
}

impl ScreenImage {
    pub fn to_rgb_buf(&self) -> [u8; IMAGE_WIDTH * IMAGE_HEIGHT * 3] {
        let mut buf: [u8; _] = [0; IMAGE_WIDTH * IMAGE_HEIGHT * 3];

        for i in 0..IMAGE_HEIGHT {
            for j in 0..IMAGE_WIDTH {
                let p = self.data[i][j].to_rgb().to_buf();
                let s = 3 * ((i * IMAGE_WIDTH) + j);
                let e = s + 3;
                buf[s..e].copy_from_slice(&p);
            }
        }

        buf
    }
}

impl From<image::DynamicImage> for ScreenImage {
    fn from(value: image::DynamicImage) -> Self {
        let mut data = [[Pixel::EMPTY; IMAGE_WIDTH]; IMAGE_HEIGHT];

        let new_img = value
            .resize_to_fill(
                IMAGE_WIDTH as u32,
                IMAGE_HEIGHT as u32,
                image::imageops::FilterType::Nearest,
            )
            .into_rgb8();

        for i in 0..IMAGE_HEIGHT {
            for j in 0..IMAGE_WIDTH {
                let p = new_img[(j as u32, i as u32)];
                data[i][j] = Pixel::RGB(PixelRGB {
                    r: p.0[0],
                    g: p.0[1],
                    b: p.0[2],
                });
            }
        }

        Self { data }
    }
}
