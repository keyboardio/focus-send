// kaleidoscope -- Talk with Kaleidoscope powered devices
// Copyright (C) 2022  Keyboard.io, Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use serialport::SerialPort;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

pub struct Focus {
    port: Box<dyn SerialPort>,
    chunk_size: usize,
    write_delay: u64,
}

impl From<Box<dyn SerialPort>> for Focus {
    fn from(port: Box<dyn SerialPort>) -> Self {
        Self {
            port,
            chunk_size: 32,
            write_delay: 500,
        }
    }
}

impl Focus {
    pub fn chunk_size(&mut self, chunk_size: usize) -> &Self {
        self.chunk_size = chunk_size;
        self
    }

    pub fn write_delay(&mut self, write_delay: u64) -> &Self {
        self.write_delay = write_delay;
        self
    }

    pub fn flush(&mut self) -> Result<(), std::io::Error> {
        self.request(String::from(" "), None)?;
        self.read_reply()?;
        Ok(())
    }

    pub fn request(
        &mut self,
        command: String,
        args: Option<Vec<String>>,
    ) -> Result<(), std::io::Error> {
        self.request_with_progress(command, args, |_| {}, |_| {})
    }

    pub fn request_with_progress<FL, FP>(
        &mut self,
        command: String,
        args: Option<Vec<String>>,
        set_length: FL,
        progress: FP,
    ) -> Result<(), std::io::Error>
    where
        FL: Fn(usize),
        FP: Fn(usize),
    {
        let request = [vec![command], args.unwrap_or_default()].concat().join(" ") + "\n";
        self.port.write_data_terminal_ready(true)?;

        set_length(request.len());

        for c in request.as_bytes().chunks(self.chunk_size) {
            progress(c.len());
            self.port.write_all(c)?;
            thread::sleep(Duration::from_millis(self.write_delay));
        }

        Ok(())
    }

    pub fn read_reply(&mut self) -> Result<String, std::io::Error> {
        let mut buffer: Vec<u8> = vec![0; 1024];
        let mut reply = vec![];

        self.port.read_data_set_ready()?;
        self.wait_for_data()?;

        loop {
            match self.port.read(buffer.as_mut_slice()) {
                Ok(t) => {
                    reply.extend(&buffer[..t]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
            }

            thread::sleep(Duration::from_millis(self.write_delay));
        }

        Ok(String::from_utf8_lossy(&reply)
            .to_string()
            .lines()
            .filter(|l| !l.is_empty() && *l != ".")
            .collect::<Vec<&str>>()
            .join("\n"))
    }

    fn wait_for_data(&mut self) -> Result<(), std::io::Error> {
        while self.port.bytes_to_read()? == 0 {
            thread::sleep(Duration::from_millis(self.write_delay));
        }
        Ok(())
    }
}
