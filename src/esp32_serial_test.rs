use serialport::{SerialPortType};
use std::io::Read;
use std::time::Duration;

fn main() {
    let baud_rate = 115200;
    let timeout = Duration::from_millis(2000);
    println!("Listing serial ports...");
    let ports = match serialport::available_ports() {
        Ok(ports) => ports,
        Err(e) => {
            eprintln!("Error listing ports: {}", e);
            return;
        }
    };
    if ports.is_empty() {
        println!("No serial ports found.");
        return;
    }
    let mut esp32_port: Option<String> = None;
    for port in &ports {
        match &port.port_type {
            SerialPortType::UsbPort(info) => {
                let product = info.product.as_deref().unwrap_or("").to_lowercase();
                let manufacturer = info.manufacturer.as_deref().unwrap_or("").to_lowercase();
                if product.contains("cp210") || product.contains("ch340") || product.contains("esp32") || manufacturer.contains("silicon") || manufacturer.contains("wch") {
                    esp32_port = Some(port.port_name.clone());
                    break;
                }
            }
            _ => {}
        }
    }
    let port_name = match esp32_port {
        Some(name) => name,
        None => {
            println!("ESP32 port not found. Available ports:");
            for port in &ports {
                println!("- {}", port.port_name);
            }
            return;
        }
    };
    println!("ESP32 detected on port: {}", port_name);
    let mut port = match serialport::new(&port_name, baud_rate).timeout(timeout).open() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to open port: {}", e);
            return;
        }
    };
    println!("Reading serial data (Ctrl+C to exit)...");
    let mut buf = [0u8; 256];
    loop {
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                let s = String::from_utf8_lossy(&buf[..n]);
                print!("{}", s);
            }
            Ok(_) => {},
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {},
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }
}
