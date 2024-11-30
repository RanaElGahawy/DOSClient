use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io;

mod show_me;
mod send_me;

use show_me::{handle_show_me_request, send_show_me_request};
use send_me::{handle_send_me_request, send_me_request};

// Type alias for the client map
type ClientMap = Arc<Mutex<HashMap<String, String>>>;

// Initialize the client map
fn initialize_client_map() -> ClientMap {
    Arc::new(Mutex::new(HashMap::new()))
}

// Add a client to the client map
async fn add_client(client_map: ClientMap, client_id: String, client_ip: String) {
    let mut map = client_map.lock().await;
    map.insert(client_id.clone(), client_ip.clone());
    println!("Client added: ID={}, IP={}", client_id, client_ip);
}

// Remove a client from the client map
async fn remove_client(client_map: ClientMap, client_id: &str) {
    let mut map = client_map.lock().await;
    if map.remove(client_id).is_some() {
        println!("Client removed: ID={}", client_id);
    } else {
        println!("Client ID {} not found.", client_id);
    }
}

// Lookup a client's IP
async fn get_client_ip(client_map: ClientMap, client_id: &str) -> Option<String> {
    let map = client_map.lock().await;
    map.get(client_id).cloned()
}

// Handles incoming requests from clients
async fn listen_for_requests(addr: &str, client_map: ClientMap) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Middleware listening on {}", addr);

    loop {
        let (mut socket, peer_addr) = listener.accept().await?;
        let client_map = client_map.clone();

        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            if let Ok(n) = socket.read(&mut buffer).await {
                let request = String::from_utf8_lossy(&buffer[..n]);
                if request.trim().starts_with("REGISTER") {
                    // Example: REGISTER client1
                    let parts: Vec<&str> = request.trim().split_whitespace().collect();
                    if parts.len() == 2 {
                        let client_id = parts[1].to_string();
                        add_client(client_map.clone(), client_id, peer_addr.to_string()).await;
                        let _ = socket.write_all(b"ACK").await;
                    }
                } else if request.trim().starts_with("SHOW_ME") {
                    if let Ok(response) = handle_show_me_request().await {
                        let _ = socket.write_all(response.as_bytes()).await;
                    }
                } else if request.trim().starts_with("SEND_ME") {
                    if let Err(e) = handle_send_me_request(request.trim(), socket).await {
                        eprintln!("Error handling 'SEND_ME' request: {}", e);
                    }
                } else {
                    eprintln!("Unknown request: {}", request);
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize the client map
    let client_map = initialize_client_map();

    // Start the middleware
    let middleware_addr = "127.0.0.1:8080";
    listen_for_requests(middleware_addr, client_map.clone()).await
}
