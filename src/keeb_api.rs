use std::{error::Error, time::Duration};

use log::debug;
use serialport::SerialPort;

const VID: u16 = 0x514B;
const PID: u16 = 0x4D04;
const BAUD: u32 = 115200;
const BUFFER_SIZE: usize = 64;

enum Command {
    MatrixLightingSetInfo = 0xc0,
    MatrixLightingSetBuffer = 0xc1,
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

    pub fn set_matrix_lighting(&mut self, data: &[[u8; 7]; 7]) -> Result<(), Box<dyn Error>> {
        let buf: Vec<_> = data
            .iter()
            .flat_map(|r| r.iter().flat_map(|b| [191, 255, b * 255]))
            .collect();

        self.set_matrix_lighting_impl(1, 10, 7, 7, &buf)
    }
}
