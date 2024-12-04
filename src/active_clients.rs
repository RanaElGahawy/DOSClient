use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use std::io;

pub async fn show_active_clients(
    server_addr: &str,
    active_clients: Arc<Mutex<HashMap<String, String>>>,
) -> io::Result<()> {
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await??;
    println!("Connected to server.");
    // Send SHOW_ACTIVE_CLIENTS request
    timeout(Duration::from_secs(5), socket.write_all(b"SHOW_ACTIVE_CLIENTS")).await??;
    println!("Request to show active clients sent.");
    // Read the server's response
    let mut buffer = vec![0u8; 1024];
    let n = timeout(Duration::from_secs(5), socket.read(&mut buffer)).await??;
    let response = String::from_utf8_lossy(&buffer[..n]).to_string();
    println!("Response received: {}", response);
    // Parse response into a HashMap
    let parsed_clients: HashMap<String, String> = serde_json::from_str(&response)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    // Update the shared HashMap
    let mut clients = active_clients.lock().await; // Asynchronously acquire the lock
    *clients = parsed_clients;
    println!("Active clients updated successfully.");
    Ok(())
}
