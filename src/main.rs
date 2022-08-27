// focus-send -- Bare-bones Focus testing tool
// Copyright (C) 2022  Keyboard.io, Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use clap::Parser;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use serialport::SerialPort;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short = 'd', value_name = "PATH", default_value = "/dev/ttyACM0")]
    device: String,

    command: String,
    args: Vec<String>
}

fn main() {
    let opts = Cli::parse();

    let mut port = serialport::new(&opts.device, 11520)
        .open()
        .unwrap_or_else(|e| {
            eprintln!("Failed to open \"{}\". Error: {}", opts.device, e);
            ::std::process::exit(1);
        });

    send_request(&mut port, opts.command, opts.args);

    wait_for_data(&port);

    let reply = read_reply(&mut port);
    println!("{}", reply);
}

fn send_request(port: &mut Box<dyn SerialPort>, command: String, args: Vec<String>) {
    let mut request_parts = vec![command];
    request_parts.extend(args);
    let request = request_parts.join(" ") + "\n";

    port.write_all(request.as_bytes()).unwrap_or_else(|e| {
        eprintln!("{:?}", e);
        ::std::process::exit(1);
    });
}

fn wait_for_data(port: &Box<dyn SerialPort>) {
    while port.bytes_to_read().expect("Error calling bytes_to_read") == 0 {
        thread::sleep(Duration::from_millis(100));
    }
}

fn read_reply(port: &mut Box<dyn SerialPort>) -> String {
    let mut buffer: Vec<u8> = vec![0; 1024];
    let mut result: String = String::from("");
    loop {
        match port.read(buffer.as_mut_slice()) {
            Ok(t) => {
                result = result + &String::from_utf8_lossy(&buffer[..t]).to_string();
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                break;
            },
            Err(e) => {
                eprintln!("{:?}", e);
                std::process::exit(1);
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
    result
}
