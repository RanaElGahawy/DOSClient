use tokio::time::Duration;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::fs;
use std::path::Path;

pub async fn perform_image_encryption(
    server_addr: &str,
    image_path: &str,
    save_folder: &str,
    timeout_duration: Duration,
) -> io::Result<()> {
    // Validate the image path
    if image_path.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Image path cannot be empty."));
    }

    // Step 1: Send "ENCRYPTION" request to the server
    let mut socket = send_encryption_request(server_addr).await?;

    // Step 2: Wait for server's acknowledgment (ACK)
    wait_for_encryption_acknowledgment(&mut socket).await?;

    // Step 3: Send the image to the server
    send_image_to_server(&mut socket, image_path).await?;

    println!("Image sent for encryption successfully.");

    // Step 4: Wait to receive the encrypted image
    let file_name = std::path::Path::new(image_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("encrypted_image.png");

    let save_path = format!("{}/{}", save_folder, file_name);

    tokio::select! {
        response = receive_encrypted_image(&mut socket, &save_path) => {
            response.map(|_| {
                println!("Encrypted image received and saved to {}", save_path);
            })
        },
        _ = tokio::time::sleep(timeout_duration) => {
            println!("Waiting for image encryption timed out.");
            Err(io::Error::new(io::ErrorKind::TimedOut, "Encryption timeout"))
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
    // Read the image file into a buffer
    let mut file = tokio::fs::File::open(image_path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;

    // Send the length of the image data (4 bytes)
    let data_length = buffer.len() as u32;
    socket.write_all(&data_length.to_be_bytes()).await?;

    // Send the image data in chunks
    let chunk_size = 1024;
    let mut start = 0;
    while start < buffer.len() {
        let end = std::cmp::min(start + chunk_size, buffer.len());
        let chunk = &buffer[start..end];
        socket.write_all(chunk).await?;
        socket.flush().await?;
        start = end;
    }

    println!("Image data sent successfully!");

    Ok(())
}

async fn receive_encrypted_image(socket: &mut TcpStream, save_path: &str) -> io::Result<()> {
    // Extract the folder path from the save path
    let folder = Path::new(save_path).parent().unwrap_or_else(|| Path::new(""));

    // Ensure the folder exists
    if !folder.exists() {
        fs::create_dir_all(folder).await?;
    }

    // Open a file to save the encrypted image
    let mut encrypted_file = tokio::fs::File::create(save_path).await?;

    // Receive the encrypted image in chunks
    let mut buffer = [0u8; 1024];
    loop {
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            break; // Server closed the connection
        }
        encrypted_file.write_all(&buffer[..n]).await?;
    }

    println!("Encrypted image received and saved at: {}", save_path);

    Ok(())
}

