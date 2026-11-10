mod keeb_api;

use std::error::Error;

use crate::keeb_api::KeebApi;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut k = KeebApi::new()?;

    let data = [
        [0, 0, 0, 0, 0, 0, 0],
        [0, 0, 1, 0, 1, 0, 0],
        [0, 1, 1, 1, 1, 1, 0],
        [0, 1, 1, 1, 1, 1, 0],
        [0, 0, 1, 1, 1, 0, 0],
        [0, 0, 0, 1, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0],
    ];

    k.set_matrix_lighting(&data)?;

    Ok(())
}
