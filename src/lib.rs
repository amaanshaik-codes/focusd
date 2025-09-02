// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use serialport::SerialPortType;
use std::time::Duration;
use std::io::Read;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn list_serial_ports() -> Vec<String> {
    match serialport::available_ports() {
        Ok(ports) => ports.into_iter().map(|p| p.port_name).collect(),
        Err(_) => vec![],
    }
}

#[tauri::command]
fn find_esp32_port() -> Option<String> {
    if let Ok(ports) = serialport::available_ports() {
        for port in ports {
            match &port.port_type {
                SerialPortType::UsbPort(info) => {
                    // Heuristic: ESP32 often shows up as "Silicon Labs", "CP210x", "CH340", etc.
                    let product = info.product.as_deref().unwrap_or("").to_lowercase();
                    let manufacturer = info.manufacturer.as_deref().unwrap_or("").to_lowercase();
                    if product.contains("cp210") || product.contains("ch340") || product.contains("esp32") || manufacturer.contains("silicon") || manufacturer.contains("wch") {
                        return Some(port.port_name);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[tauri::command]
fn read_esp32_serial(port_name: String, timeout_ms: Option<u64>) -> Result<String, String> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(2000));
    let baud_rate = 115200;
    match serialport::new(&port_name, baud_rate)
        .timeout(timeout)
        .open()
    {
        Ok(mut port) => {
            let mut buf = [0u8; 256];
            match port.read(&mut buf) {
                Ok(n) => {
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    Ok(s)
                }
                Err(e) => Err(format!("Read error: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to open port: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![greet, list_serial_ports, find_esp32_port, read_esp32_serial])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
