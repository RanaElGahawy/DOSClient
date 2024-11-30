use std::env;
use std::io::{self, Write};
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

    loop {
        println!("Enter 1 to register, 2 to sign out:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "1" => {
                match register_with_server(server_addr).await {
                    Ok(client_id) => println!("Client registered with ID: {}", client_id),
                    Err(e) => eprintln!("Failed to register with server: {}", e),
                }
            }
            "2" => {
                loop {
                    match sign_out(server_addr).await {
                        Ok(response) => {
                            if response.trim() == "ACK" {
                                println!("Sign out successful. Terminating program.");
                                return Ok(());
                            } else {
                                eprintln!("Sign out not acknowledged (NAK). Retrying...");
                            }
                        }
                        Err(e) => eprintln!("Failed to sign out: {}", e),
                    }
                }
            }
            _ => println!("Invalid input. Please enter 1 or 2."),
        }
    }
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

async fn sign_out(server_addr: &str) -> io::Result<String> {
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await??;

    println!("Connected to server.");

    // Send sign-out request
    timeout(Duration::from_secs(5), socket.write_all(b"SIGN_OUT")).await??;
    println!("Sign out request sent.");

    // Read the acknowledgment from the server
    let mut buffer = [0u8; 128];
    let n = timeout(Duration::from_secs(5), socket.read(&mut buffer)).await??;

    let ack = String::from_utf8_lossy(&buffer[..n]).to_string();

    println!("Sign out status: {}", ack);
    Ok(ack)
}
