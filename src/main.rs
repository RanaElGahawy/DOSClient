use std::env;
use std::io::{self};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: client <server_ip:port>");
        return Ok(());
    }

    let server_addr = &args[1];

    if !server_addr.contains(':') {
        eprintln!("Invalid server address. Expected format: <ip>:<port>");
        return Ok(());
    }

    println!("Connecting to server at: {}", server_addr);

    // Register with the server and get a unique client ID
    match register_with_server(server_addr).await {
        Ok(client_id) => println!("Client registered with ID: {}", client_id),
        Err(e) => eprintln!("Failed to register with server: {}", e),
    }

    Ok(())
}

async fn register_with_server(server_addr: &str) -> io::Result<String> {
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await??;

    println!("Connected to server.");

    // Send registration request
    timeout(Duration::from_secs(5), socket.write_all(b"JOIN")).await??;
    println!("Registration request sent.");

    // Read the assigned unique client ID from the server
    let mut buffer = [0u8; 128];
    let n = timeout(Duration::from_secs(5), socket.read(&mut buffer)).await??;

    let client_id = String::from_utf8_lossy(&buffer[..n]).to_string();

    println!("Received client ID: {}", client_id);
    Ok(client_id)
}
