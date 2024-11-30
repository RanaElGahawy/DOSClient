use std::fs;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// Sends a "SHOW_ME" request to another client
pub async fn send_show_me_request(target_addr: &str) -> io::Result<Vec<String>> {
    let mut socket = TcpStream::connect(target_addr).await?;
    println!("Connected to target client.");

    // Send "SHOW_ME" request
    socket.write_all(b"SHOW_ME").await?;
    println!("'SHOW_ME' request sent.");

    // Read the list of image names/IDs from the target client
    let mut buffer = [0u8; 1024]; // Buffer for the response
    let n = socket.read(&mut buffer).await?;
    let response = String::from_utf8_lossy(&buffer[..n]).to_string();
    let image_list: Vec<String> = response.split(',').map(|s| s.trim().to_string()).collect();

    println!("Available images: {:?}", image_list);
    Ok::<Vec<String>, io::Error>(image_list)
}

// Handles incoming "SHOW_ME" requests
pub async fn handle_show_me_request() -> io::Result<String> {
    // List files in the "images" folder
    let files = fs::read_dir("./images")?;
    let mut image_names = Vec::new();
    for file in files {
        if let Ok(entry) = file {
            if let Some(filename) = entry.file_name().to_str() {
                image_names.push(filename.to_string());
            }
        }
    }
    Ok::<String, io::Error>(image_names.join(","))
}
