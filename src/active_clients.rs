use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use std::io;

pub async fn show_active_clients(
    servers: &Vec<&str>,
    active_clients: Arc<Mutex<HashMap<String, String>>>,
) -> io::Result<()> {
    for server_addr in servers {
        match timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await {
            Ok(Ok(mut socket)) => {
                println!("Connected to server at {}.", server_addr);

                // Send SHOW_ACTIVE_CLIENTS request
                if let Err(e) = timeout(Duration::from_secs(5), socket.write_all(b"SHOW_ACTIVE_CLIENTS")).await {
                    eprintln!("Failed to send request to {}: {}", server_addr, e);
                    continue; // Try the next server
                }
                println!("Request to show active clients sent to {}.", server_addr);

                // Read the server's response
                let mut buffer = vec![0u8; 1024];
                match timeout(Duration::from_secs(5), socket.read(&mut buffer)).await {
                    Ok(Ok(n)) => {
                        let response = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("Response received from {}: {}", server_addr, response);

                        // Parse response into a HashMap
                        match serde_json::from_str::<HashMap<String, String>>(&response) {
                            Ok(parsed_clients) => {
                                // Update the shared HashMap
                                let mut clients = active_clients.lock().await; // Acquire lock asynchronously
                                *clients = parsed_clients;

                                println!("Active clients updated successfully from {}.", server_addr);
                                return Ok(()); // Successfully updated clients
                            }
                            Err(e) => {
                                eprintln!("Failed to parse response from {}: {}", server_addr, e);
                                continue; // Try the next server
                            }
                        }
                    }
                    Ok(Err(e)) => eprintln!("Failed to read response from {}: {}", server_addr, e),
                    Err(_) => eprintln!("Timeout while reading response from {}.", server_addr),
                }
            }
            Ok(Err(_)) => {},
            Err(_) => {},
        }
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to retrieve active clients from any server",
    ))
}
