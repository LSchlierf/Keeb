mod image;
mod keeb_api;

use std::error::Error;

use ::image::ImageReader;
use clap::Parser;
use log::info;

use crate::{
    image::{Bitmap, PixelMatrixImage},
    keeb_api::KeebApi,
};

#[derive(clap::Parser)]
struct CLI {
    /// Sets the screen to an image
    #[arg(short('s'), long("screen"), value_name = "FILE")]
    screen: Option<String>,

    /// Sets the pixel matrix to the specified bitmap
    #[arg(short('b'), long("bitmap"), value_name = "PATTERN")]
    matrix: Option<Bitmap>,

    /// Sets the color for the bitmap
    #[arg(short('c'), long("color"), value_name = "COLOR")]
    color: Option<u8>,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = CLI::parse();

    let mut k = KeebApi::new()?;

    if let Some(pattern) = cli.matrix {
        let hue = cli.color.unwrap_or(0);
        info!("Setting the matrix to {:?}, color {}", pattern, hue);
        k.set_matrix_lighting(PixelMatrixImage::from_bitmap_hue(pattern.into(), hue))?;
    }

    if let Some(path) = cli.screen {
        info!("Setting the screen to {}", path);
        let img = ImageReader::open(path)?.with_guessed_format()?.decode()?;
        k.set_image(img.into())?;
    }

    Ok(())
}
