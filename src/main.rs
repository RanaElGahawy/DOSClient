use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::net::{TcpStream, UdpSocket};
use tokio::task;

mod server_registeration;
mod active_clients; // Include the new module

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

    let mut client_id = String::new();
    let active_clients = Arc::new(Mutex::new(HashMap::new())); // Shared active clients list

    // Check if client_ID file exists
    if let Ok(mut file) = File::open("client_ID") {
        let mut id = String::new();
        file.read_to_string(&mut id)?;
        client_id = id.trim().to_string();
        println!("Found existing client ID: {}", client_id);

        // Send REJOIN request
        match server_registeration::rejoin_with_server(server_addr, &client_id).await {
            Ok(response) => println!("Rejoin successful: {}", response),
            Err(e) => eprintln!("Failed to rejoin with server: {}", e),
        }
    } else {
        // No client_ID file, register with the server
        println!("No existing client ID found. Registering with the server...");
        match server_registeration::register_with_server(server_addr).await {
            Ok(id) => {
                client_id = id.clone();
                // Save the new client ID to a file
                if let Err(e) = save_client_id_to_file(&client_id) {
                    eprintln!("Failed to save client ID to file: {}", e);
                }
                println!("Client registered with ID: {}", client_id);
            }
            Err(e) => eprintln!("Failed to register with server: {}", e),
        }
    }

    // Start the UDP listener in a background task
    let _ = task::spawn(udp_listener_task());

    loop {
        println!("Enter 0 to sign out, 1 to show active clients, 2 to mark unreachable client, 3 to send an image for encryption:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "0" => {
                if client_id.is_empty() {
                    println!("You must register first before signing out.");
                } else {
                    match server_registeration::sign_out(server_addr, &client_id).await {
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
            "1" => {
                match active_clients::show_active_clients(server_addr, Arc::clone(&active_clients)).await {
                    Ok(_) => {
                        let clients = active_clients.lock().await; // Asynchronously acquire the lock
                        println!("Active clients: {:?}", *clients);
                    }
                    Err(e) => eprintln!("Failed to fetch active clients: {}", e),
                }
            }
            "2" => {
                // Option 2: Provide an unreachable client ID
                println!("Enter the ID of the client to mark as unreachable:");
                let mut unreachable_id = String::new();
                io::stdin().read_line(&mut unreachable_id)?;
                let unreachable_id = unreachable_id.trim();

                if unreachable_id.is_empty() {
                    eprintln!("Client ID cannot be empty.");
                    continue;
                }

                // Send a request to mark the client as unreachable
                match server_registeration::mark_client_unreachable(server_addr, unreachable_id).await {
                    Ok(_) => println!("Successfully marked client ID {} as unreachable", unreachable_id),
                    Err(e) => eprintln!("Failed to mark client ID {} as unreachable: {}", unreachable_id, e),
                }
            }
            "3" => {
                println!("Enter the path to the image file you want to send:");
                let mut image_path = String::new();
                io::stdin().read_line(&mut image_path)?;
                let image_path = image_path.trim();
            
                if image_path.is_empty() {
                    eprintln!("Image path cannot be empty.");
                    continue;
                }
            
                // Step 1: Send "ENCRYPTION" request to the server
                match send_encryption_request(server_addr).await {
                    Ok(mut socket) => {
                        // Step 2: Wait for server's acknowledgment (ACK)
                        match wait_for_encryption_acknowledgment(&mut socket).await {
                            Ok(_) => {
                                println!("Sending image.");
                                // Step 3: Read the image file and send it to the server
                                match send_image_to_server(&mut socket, image_path).await {
                                    Ok(_) => {
                                        println!("Image sent for encryption successfully.");
            
                                        // Step 4: Wait for the encrypted image or handle timeout
                                        let _ = tokio::select! {
                                            response = receive_encrypted_image(socket) => {
                                                match response {
                                                    Ok(_) => println!("Encrypted image received."),
                                                    Err(e) => eprintln!("Error receiving encrypted image: {}", e),
                                                }
                                            },
                                            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                                                println!("Waiting for image encryption timed out.");
                                            }
                                        };
                                    }
                                    Err(e) => eprintln!("Failed to send image: {}", e),
                                }
                            }
                            Err(e) => eprintln!("Failed to receive acknowledgment from server: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to send encryption request: {}", e),
                }
            }

            _ => println!("Invalid input. Please enter 0 to sign out or 1 to show active clients."),
        }
    }
}

async fn wait_for_encryption_acknowledgment(socket: &mut TcpStream) -> io::Result<()> {
    let mut buffer = vec![0u8; 1024];

    // Wait for the server to acknowledge the ENCRYPTION command
    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "No acknowledgment from server"));
    }

    // Check for a specific acknowledgment message from the server
    let response = String::from_utf8_lossy(&buffer[..n]);
    if response.trim() == "ACK" {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Unexpected response from server"))
    }
}

// Function to send the "ENCRYPTION" request
async fn send_encryption_request(server_addr: &str) -> io::Result<TcpStream> {
    let mut socket = TcpStream::connect(server_addr).await?;
    let encryption_request = "ENCRYPTION";
    socket.write_all(encryption_request.as_bytes()).await?;
    socket.flush().await?;  // Ensure the message is sent
    Ok(socket)
}

async fn send_image_to_server(socket: &mut TcpStream, image_path: &str) -> io::Result<()> {
    let mut file = File::open(image_path)?;
    let mut image_data = Vec::new();
    file.read_to_end(&mut image_data)?;

    // Send the image data over the same socket in chunks of 1024 bytes
    let chunk_size = 1024;
    let mut start = 0;
    while start < image_data.len() {
        let end = std::cmp::min(start + chunk_size, image_data.len());
        let chunk = &image_data[start..end];
        socket.write_all(chunk).await?;
        socket.flush().await?;  // Ensure each chunk is sent immediately
        start = end;
    }

    Ok(())
}


// Function to receive the encrypted image from the server
async fn receive_encrypted_image(mut socket: TcpStream) -> io::Result<()> {
    let mut buffer = vec![0u8; 1024]; // Adjust the buffer size as needed

    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "No data received"));
    }

    // Save the received encrypted image
    let encrypted_image_path = "encrypted_image.png"; // Customize path as necessary
    let mut encrypted_file = tokio::fs::File::create(encrypted_image_path).await?;
    encrypted_file.write_all(&buffer[..n]).await?;

    Ok(())
}


async fn udp_listener_task() {
    let socket = UdpSocket::bind("0.0.0.0:12345").await.unwrap(); // Bind to a local port to listen
    let mut buf = [0; 1024]; // Buffer to hold incoming data

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n, addr)) => {
                let received_message = String::from_utf8_lossy(&buf[..n]);
                if received_message == "PING" {
                    // Respond with "ACK" when a PING message is received
                    if let Err(e) = socket.send_to(b"ACK", addr).await {
                        eprintln!("Failed to send ACK: {}", e);
                    } else {
                        println!("Received PING from {}. Responding with ACK.", addr);
                    }
                }
            }
            Err(e) => eprintln!("Error receiving UDP packet: {}", e),
        }
    }
}

fn save_client_id_to_file(client_id: &str) -> io::Result<()> {
    let mut file = File::create("client_ID")?;
    file.write_all(client_id.as_bytes())?;
    Ok(())
}
