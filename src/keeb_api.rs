use std::{error::Error, time::Duration};

use crate::image::IMAGE_HEIGHT;
use crate::image::IMAGE_WIDTH;
use crate::image::PixelMatrixImage;
use crate::image::ScreenImage;

use log::debug;
use map_to_range::MapRange;
use serialport::SerialPort;

const VID: u16 = 0x514B;
const PID: u16 = 0x4D04;
const BAUD: u32 = 115200;
const BUFFER_SIZE: usize = 64;

enum Command {
    MatrixLightingSetInfo = 0xc0,
    MatrixLightingSetBuffer = 0xc1,
    ImageFileSetInfo = 0xe0,
    ImageFileSetBuffer = 0xe1,
    // ImageFileSetCancel = 0xe2,
}

pub(crate) struct KeebApi {
    device: Box<dyn SerialPort>,
}

impl KeebApi {
    pub fn new() -> Result<KeebApi, Box<dyn Error>> {
        let ports = serialport::available_ports()?;

        let port = ports
            .iter()
            .find_map(|p| match &p.port_type {
                serialport::SerialPortType::UsbPort(info) if info.pid == PID && info.vid == VID => {
                    Some(p)
                }
                _ => None,
            })
            .ok_or(serialport::Error::new(
                serialport::ErrorKind::NoDevice,
                "Could not find keyboard",
            ))?;

        let device = serialport::new(port.port_name.clone(), BAUD)
            .timeout(Duration::from_millis(100))
            .open()?;

        Ok(KeebApi { device })
    }

    fn serial_command(&mut self, command: Command, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        debug_assert!(bytes.len() <= 63);
        let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
        buffer[0] = command as u8;
        for i in 1..BUFFER_SIZE.min(bytes.len() + 1) {
            buffer[i] = bytes[i - 1];
        }

        debug!("writing: {:02X?}", buffer);

        let n = self.device.write(&buffer)?;

        debug!("wrote {n} bytes");

        Ok(())
    }

    fn serial_read(&mut self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut resp: Vec<u8> = vec![];
        resp.resize(64, 0);

        let n = self.device.read(&mut resp)?;

        resp.resize(n, 0);

        debug!("read {} bytes: {:02X?}", n, resp);
        Ok(resp)
    }

    fn set_matrix_lighting_impl(
        &mut self,
        frames: u8,
        fps: u8,
        rows: u8,
        cols: u8,
        data: &[u8],
    ) -> Result<(), Box<dyn Error>> {
        self.serial_command(Command::MatrixLightingSetInfo, &[frames, fps, rows, cols])?;

        let resp = self.serial_read()?;

        if resp.len() > 4 && resp[5] == 0xee {
            debug!("error");
            return Ok(());
        }

        let mut offset = 0;
        let batch_size = BUFFER_SIZE - 8;

        debug!("Data is {:02X?}", data);

        while offset < data.len() {
            let start = offset;
            let end = offset + batch_size;
            let buf = &data[start..(end.min(data.len()))];

            let buffer: Vec<u8> = (offset as u32)
                .to_be_bytes()
                .iter()
                .chain(&[buf.len() as u8])
                .chain(buf)
                .map(|b| *b)
                .collect();

            debug!(
                "issuing offset 0x{:X} length {} (0x{:X})",
                offset,
                buf.len(),
                buf.len()
            );

            debug!("{:02X?}", buffer);

            self.serial_command(Command::MatrixLightingSetBuffer, &buffer)?;

            offset += batch_size;
        }

        self.serial_read().unwrap_err();

        Ok(())
    }

    pub fn set_matrix_lighting(&mut self, data: PixelMatrixImage) -> Result<(), Box<dyn Error>> {
        let buf = data.to_hsl_buf();

        self.set_matrix_lighting_impl(1, 10, 7, 7, &buf)
    }

    fn pack_image(data: &[u8; IMAGE_WIDTH * IMAGE_HEIGHT * 3]) -> [u8; IMAGE_WIDTH * IMAGE_HEIGHT * 2] {
        // 16b per pixel
        //
        // wht 0xFFFF 0b1111111111111111
        // blk 0x0000 0b0000000000000000
        // red 0x00F8 0b0000000011111000
        // grn 0xE007 0b1110000000000111
        // blu 0x1F00 0b0001111100000000
        //
        // mgt 0x1FF8 0b0001111111111000
        // ylw 0xE0FF 0b1110000011111111
        // cyn 0xFF07 0b1111111100000111
        //
        // 6 "outer" bits for green
        // 5 bits for red, blue each

        let mut packed: [u8; _] = [0; IMAGE_WIDTH * IMAGE_HEIGHT * 2];

        for i in 0..(IMAGE_WIDTH * IMAGE_HEIGHT) {
            let r = data[i * 3];
            let g = data[i * 3 + 1];
            let b = data[i * 3 + 2];
            let rp = r.map_range((0, 0xFF), (0, 0x1F)).unwrap();
            let gp = g.map_range((0, 0xFF), (0, 0x3F)).unwrap(); // g has 6 bits instead of 4
            let bp = b.map_range((0, 0xFF), (0, 0x1F)).unwrap();
            packed[i * 2] = bp | (gp << 5);
            packed[i * 2 + 1] = (rp << 3) | (gp >> 3);
        }

        packed
    }

    fn set_image_impl(
        &mut self,
        data: [u8; IMAGE_WIDTH * IMAGE_HEIGHT * 2],
    ) -> Result<(), Box<dyn Error>> {
        let mut payload: Vec<u8> = vec![0x41, 0x42, 0x4B, 0x54, 0x14, 0x00, 0x14, 0x00, 0x14, 0xAE, 0x01, 0x00, 0x40, 0x01, 0xAC, 0x00, 0x00, 0x00, 0x01, 0x00];

        self.serial_command(Command::ImageFileSetInfo, &payload)?;

        let resp = self.serial_read()?;

        if resp.len() > 21 && resp[21] == 0xEE {
            debug!("Error");
            return Ok(())
        }

        payload.extend_from_slice(&data);

        let mut offset = 0;
        let batch_size = BUFFER_SIZE - 8;

        while offset < payload.len() {
            let start = offset;
            let end = offset + batch_size;
            let buf = &payload[start..(end.min(payload.len()))];

            let buffer: Vec<u8> = (offset as u32)
                .to_be_bytes()
                .iter()
                .chain(&[buf.len() as u8])
                .chain(buf)
                .map(|b| *b)
                .collect();

            debug!(
                "issuing offset 0x{:X} length {} (0x{:X})",
                offset,
                buf.len(),
                buf.len()
            );

            debug!("{:02X?}", buffer);

            self.serial_command(Command::ImageFileSetBuffer, &buffer)?;

            offset += batch_size;
        }

        self.serial_read().unwrap_err();

        Ok(())
    }

    pub fn set_image(
        &mut self,
        image: ScreenImage
    ) -> Result<(), Box<dyn Error>> {
        self.set_image_impl(Self::pack_image(&image.to_rgb_buf()))
    }
}
