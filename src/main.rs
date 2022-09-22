// focus-send -- Bare-bones Focus testing tool
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

use clap::Parser;
use indicatif::ProgressBar;
use serialport::SerialPort;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(
        short,
        long,
        env,
        hide_env = true,
        value_name = "PATH",
        help = "The device to connect to"
    )]
    device: Option<String>,
    #[arg(short, long, help = "Operate quietly", default_value = "false")]
    quiet: bool,

    command: String,
    args: Vec<String>,
}

fn main() {
    let opts = Cli::parse();
    let device = opts.device().unwrap_or_else(|| {
        eprintln!("No device found to connect to");
        ::std::process::exit(1);
    });

    let mut port = serialport::new(&device, 11520)
        .timeout(Duration::from_millis(100))
        .open()
        .unwrap_or_else(|e| {
            eprintln!("Failed to open \"{}\". Error: {}", &device, e);
            ::std::process::exit(1);
        });

    flush(&mut port);

    send_request(&mut port, !opts.quiet, opts.command, opts.args)
        .expect("failed to send the request to the keyboard");

    wait_for_data(&*port);

    let reply = read_reply(&mut port).expect("failed to read the reply");
    println!("{}", reply);
}

impl Cli {
    fn device(&self) -> Option<String> {
        #[derive(PartialEq)]
        struct DeviceDescriptor {
            vid: u16,
            pid: u16,
        }
        let supported_keyboards = [
            // Keyboardio Model100
            DeviceDescriptor {
                vid: 0x3496,
                pid: 0x0006,
            },
            // Keyboardio Atreus
            DeviceDescriptor {
                vid: 0x1209,
                pid: 0x2303,
            },
            // Keyboardio Model01
            DeviceDescriptor {
                vid: 0x1209,
                pid: 0x2301,
            },
        ];

        // If we had a device explicitly specified, use that.
        if let Some(device) = &self.device {
            return Some(device.to_string());
        }

        // Otherwise list the serial ports, and return the first USB serial port
        // that has a vid/pid that matches any of the Keyboardio devices.
        serialport::available_ports()
            .ok()?
            .iter()
            .filter_map(|p| match &p.port_type {
                serialport::SerialPortType::UsbPort(port_info) => {
                    struct MinimalPortInfo {
                        ids: DeviceDescriptor,
                        port: String,
                    }
                    Some(MinimalPortInfo {
                        ids: DeviceDescriptor {
                            vid: port_info.vid,
                            pid: port_info.pid,
                        },
                        port: p.port_name.to_string(),
                    })
                }
                _ => None,
            })
            .find_map(|p| supported_keyboards.contains(&p.ids).then(|| p.port))
    }
}

// Send an empty command, and consume any replies. This should clear any pending
// commands or output.
fn flush(port: &mut Box<dyn SerialPort>) {
    send_request(port, false, String::from(" "), vec![]).expect("failed to send an empty command");
    wait_for_data(&**port);
    read_reply(port).expect("failed to flush the device");
}

fn send_request(
    port: &mut Box<dyn SerialPort>,
    with_progress: bool,
    command: String,
    args: Vec<String>,
) -> Result<(), std::io::Error> {
    let request = [vec![command], args.clone()].concat().join(" ") + "\n";

    port.write_data_terminal_ready(true)?;

    let pb = if with_progress && !args.is_empty() {
        ProgressBar::new(request.len().try_into().unwrap())
    } else {
        ProgressBar::hidden()
    };

    for c in request.as_bytes().chunks(64) {
        pb.inc(c.len().try_into().unwrap());
        port.write_all(c)?;
        thread::sleep(Duration::from_millis(50));
    }

    pb.finish_and_clear();
    Ok(())
}

fn wait_for_data(port: &dyn SerialPort) {
    while port.bytes_to_read().expect("Error calling bytes_to_read") == 0 {
        thread::sleep(Duration::from_millis(100));
    }
}

fn read_reply(port: &mut Box<dyn SerialPort>) -> Result<String, std::io::Error> {
    let mut buffer: Vec<u8> = vec![0; 1024];
    let mut reply = vec![];

    port.read_data_set_ready()?;

    loop {
        match port.read(buffer.as_mut_slice()) {
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

        thread::sleep(Duration::from_millis(100));
    }

    Ok(cleanup_reply(String::from_utf8_lossy(&reply).to_string()))
}

fn cleanup_reply(reply: String) -> String {
    reply
        .lines()
        .filter(|l| !l.is_empty() && *l != ".")
        .collect::<Vec<&str>>()
        .join("\n")
}

#[cfg(test)]
mod test {
    #[test]
    fn cleanup_reply() {
        assert_eq!(
            super::cleanup_reply(String::from("line1\nline2\r\nline3")),
            "line1\nline2\nline3"
        );
    }
}
