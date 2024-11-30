use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::io;
use std::path::Path;
use crate::encryption::encode_image_with_hidden; // Import encryption function

// Sends a "SEND_ME" request to another client
pub async fn send_me_request(target_addr: &str, image_names: Vec<&str>) -> io::Result<()> {
    let mut socket = TcpStream::connect(target_addr).await?;
    println!("Connected to target client.");

    // Send the "SEND_ME" request with the image names
    let request = format!("SEND_ME {}", image_names.join(","));
    socket.write_all(request.as_bytes()).await?;
    println!("'SEND_ME' request sent for images: {:?}", image_names);

    // Receive the encrypted images
    for image_name in &image_names {
        let mut buffer = Vec::new();
        socket.read_to_end(&mut buffer).await?;

        // Save the received encrypted image in the "borrowed_images" folder
        let file_path = format!("./borrowed_images/{}", image_name);
        tokio::fs::create_dir_all("./borrowed_images").await?; // Ensure folder exists
        tokio::fs::write(&file_path, &buffer).await?;
        println!(
            "Encrypted image '{}' received and saved to 'borrowed_images' folder.",
            image_name
        );
    }

    Ok(())
}

// Handles incoming "SEND_ME" requests
pub async fn handle_send_me_request(request: &str, mut socket: TcpStream) -> io::Result<()> {
    // Extract image names from the request
    let image_names: Vec<&str> = request
        .trim_start_matches("SEND_ME ")
        .split(',')
        .map(|name| name.trim())
        .collect();

    for image_name in &image_names {
        let image_path = format!("./images/{}", image_name);

        if Path::new(&image_path).exists() {
            // Encrypt the image with default access rights (5)
            let encoded_image_path = encode_image_with_hidden(&image_path, 5).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Error encrypting image: {:?}", e))
            })?;

            // Read the encrypted image
            let mut file = tokio::fs::File::open(&encoded_image_path).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;

            // Send the encrypted image back to the requester
            socket.write_all(&buffer).await?;
            println!("Encrypted image '{}' sent to the requester.", image_name);
        } else {
            eprintln!("Image '{}' not found in the 'images' folder.", image_name);
        }
    }

    Ok(())
}