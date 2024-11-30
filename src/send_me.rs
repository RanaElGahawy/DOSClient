use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::io;
use std::path::Path;
use crate::encryption::{encode_image_with_hidden}; // Import the encryption function

// Sends a "SEND_ME" request to another client
pub async fn send_me_request(target_addr: &str, image_names: Vec<&str>) -> io::Result<()> {
    let mut socket = TcpStream::connect(target_addr).await?;
    println!("Connected to target client.");

    // Format the "SEND_ME" request with the image names
    let request = format!("SEND_ME {}", image_names.join(","));
    socket.write_all(request.as_bytes()).await?;
    println!("'SEND_ME' request sent for images: {:?}", image_names);

    for image_name in &image_names {
        let path = format!("./images/{}", image_name);
        if Path::new(&path).exists() {
            // Prompt the user for the number of access rights
            println!(
                "Specify the number of access rights for '{}': (Default = 5)",
                image_name
            );
            let mut access_rights_input = String::new();
            io::stdin().read_line(&mut access_rights_input)?;
            let access_rights: u8 = access_rights_input.trim().parse().unwrap_or(5);

            // Encrypt the image with the specified access rights
            let encoded_image_path = encode_image_with_hidden(&path, access_rights).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Error encrypting image: {:?}", e))
            })?;

            let mut file = tokio::fs::File::open(encoded_image_path).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;
            socket.write_all(&buffer).await?;
            println!(
                "Encrypted image '{}' with {} access rights sent to the target client.",
                image_name, access_rights
            );
        } else {
            eprintln!("Image '{}' not found in the 'images' folder.", image_name);
        }
    }

    Ok(())
}

// Handles incoming "SEND_ME" requests
pub async fn handle_send_me_request(request: &str, mut socket: TcpStream) -> io::Result<()> {
    let image_names: Vec<&str> = request
        .trim_start_matches("SEND_ME ")
        .split(',')
        .map(|name| name.trim())
        .collect();

    for image_name in image_names {
        let mut buffer = Vec::new();

        // Receive the encrypted image data
        socket.read_to_end(&mut buffer).await?;

        // Save the received image to the "borrowed_images" folder
        let file_path = format!("./borrowed_images/{}", image_name);
        tokio::fs::create_dir_all("./borrowed_images").await?;
        tokio::fs::write(&file_path, &buffer).await?;
        println!(
            "Encrypted image '{}' received and saved to 'borrowed_images' folder.",
            image_name
        );
    }

    Ok(())
}