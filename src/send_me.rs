use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::io;
use std::path::Path;
use crate::encryption::encode_image_with_hidden;

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

// Handles "SEND_ME" requests with access rights prompting
pub async fn handle_send_me_request_with_prompt(request: &str, mut socket: TcpStream) -> io::Result<()> {
    let image_names: Vec<&str> = request
        .trim_start_matches("SEND_ME ")
        .split(',')
        .map(|name| name.trim())
        .collect();

    for image_name in &image_names {
        let image_path = format!("./images/{}", image_name);

        if Path::new(&image_path).exists() {
            let access_rights = prompt_access_rights(image_name).await?;

            let encoded_image_path = encode_image_with_hidden(&image_path, access_rights).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Error encrypting image: {:?}", e))
            })?;

            let mut file = tokio::fs::File::open(&encoded_image_path).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;
            socket.write_all(&buffer).await?;
            println!("Encrypted image '{}' sent to the requester.", image_name);
        } else {
            eprintln!("Image '{}' not found in the 'images' folder.", image_name);
        }
    }
    Ok(())
}



// Prompt for access rights during request handling
async fn prompt_access_rights(image_name: &str) -> io::Result<u8> {
    loop {
        println!("\nEnter access rights for the image '{}':", image_name);
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim().parse::<u8>() {
            Ok(rights) if rights >= 1 && rights <= 5 => {
                println!("Access rights '{}' accepted.", rights);
                return Ok(rights);
            }
            _ => println!("Invalid access rights. Please enter a value between 1 and 5."),
        }
    }
}