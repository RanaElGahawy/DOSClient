use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

pub async fn register_with_server(server_addr: &str) -> io::Result<String> {
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

pub async fn rejoin_with_server(server_addr: &str, client_id: &str) -> io::Result<String> {
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await??;

    println!("Connected to server.");

    // Send rejoin request
    let rejoin_message = format!("REJOIN {}", client_id);
    timeout(Duration::from_secs(5), socket.write_all(rejoin_message.as_bytes())).await??;
    println!("Rejoin request sent with ID: {}", client_id);

    // Read the server's response
    let mut buffer = [0u8; 128];
    let n = timeout(Duration::from_secs(5), socket.read(&mut buffer)).await??;

    let response = String::from_utf8_lossy(&buffer[..n]).to_string();

    println!("Rejoin response: {}", response);
    Ok(response)
}

pub async fn sign_out(server_addr: &str, client_id: &str) -> io::Result<String> {
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await??;

    println!("Connected to server.");

    // Send sign-out request with client ID
    let sign_out_message = format!("SIGN_OUT {}", client_id);
    timeout(Duration::from_secs(5), socket.write_all(sign_out_message.as_bytes())).await??;
    println!("Sign out request sent with ID: {}", client_id);

    // Read the acknowledgment from the server
    let mut buffer = [0u8; 128];
    let n = timeout(Duration::from_secs(5), socket.read(&mut buffer)).await??;

    let ack = String::from_utf8_lossy(&buffer[..n]).to_string();

    println!("Sign out status: {}", ack);
    Ok(ack)
}
