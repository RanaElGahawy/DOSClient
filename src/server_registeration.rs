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


fn parse_update_message(message: &str) -> Option<(String, u8)> {
    let parts: Vec<&str> = message.trim().split_whitespace().collect();
    if parts.len() == 3 && parts[0] == "UPDATE" {
        let image_path = parts[1].to_string();
        if let Ok(access_rights) = parts[2].parse::<u8>() {
            return Some((image_path, access_rights));
        }
    }
    None
}

pub async fn update_access_rights(image_path: &str, new_access_rights: u8) -> io::Result<()> {
    // Open the image
    let mut image = match image::open(image_path) {
        Ok(img) => img.to_rgba(),
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to open image: {}", e),
            ))
        }
    };


    let (_, height) = image.dimensions();
    let pixel = image.get_pixel_mut(0, height - 1);
    pixel[0] = new_access_rights; // Update the red channel with the new access rights

    // Save the updated image
    if let Err(e) = image.save(image_path) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to save updated image: {}", e),
        ));
    }

    println!("Access rights updated for image: {}", image_path);
    Ok(())
}


pub async fn rejoin_with_server(server_addr: &str, client_id: &str) -> io::Result<()> {
    // Connect to the server
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await??;
    println!("Connected to server.");

    // Send rejoin request
    let rejoin_message = format!("REJOIN {}", client_id);
    timeout(Duration::from_secs(5), socket.write_all(rejoin_message.as_bytes())).await??;
    socket.flush().await?;
    println!("Rejoin request sent with ID: {}", client_id);

    // Read the server's response
    let mut buffer = [0u8; 128];
    let n = timeout(Duration::from_secs(5), socket.read(&mut buffer)).await??;

    let response = String::from_utf8_lossy(&buffer[..n]).to_string();
    println!("Rejoin response: {}", response);

    // Parse the response to get the number of updates
    let parts: Vec<&str> = response.split_whitespace().collect();
    let update_count: usize = if parts.len() > 1 {
        parts[1].parse().unwrap_or(0)
    } else {
        0
    };
    println!("Expecting {} updates from the server.", update_count);

    // If there are updates, notify the server and listen for them
    if update_count > 0 {
        println!("Sending READY_FOR_UPDATES to the server...");
        socket.write_all(b"READY_FOR_UPDATES\n").await?;
        socket.flush().await?;

        // Listen for updates
        let mut buffer = vec![0u8; 1024];
        for _ in 0..update_count {
            match socket.read(&mut buffer).await {
                Ok(0) => {
                    println!("Connection closed by server.");
                    break;
                }
                Ok(n) => {
                    let message = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                    println!("Received message: {}", message);

                    // Handle the "UPDATE" message
                    if let Some((image_id, new_access_rights)) = parse_update_message(&message) {
                        let image_path = format!("./borrowed_images/{}", image_id);
                        match update_access_rights(&image_path, new_access_rights).await {
                            Ok(_) => println!(
                                "Access rights for '{}' updated to '{}'.",
                                image_id, new_access_rights
                            ),
                            Err(e) => eprintln!(
                                "Failed to update access rights for '{}': {}",
                                image_id, e
                            ),
                        }
                    } else {
                        println!("Unknown message format: {}", message);
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from server: {}", e);
                    break;
                }
            }
        }
    } else {
        println!("No updates from the server.");
    }

    Ok(())
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


pub async fn mark_client_unreachable(server_addr: &str, client_id: &str) -> io::Result<()> {
    // Try to connect to the server within the timeout duration
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await??;
    println!("Connected to server.");
    // Send the "UNREACHABLE" message to the server
    let unreachable_message = format!("UNREACHABLE {}", client_id);
    timeout(Duration::from_secs(5), socket.write_all(unreachable_message.as_bytes())).await??;
    println!("Unreachable request sent with ID: {}", client_id);

    // No need to read the response
    Ok(())
}
