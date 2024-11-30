use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// Sends a "SEND_ME" request to another client
pub async fn send_me_request(target_addr: &str, image_names: Vec<&str>) -> io::Result<()> {
    let mut socket = TcpStream::connect(target_addr).await?;
    println!("Connected to target client.");

    // Format the "SEND_ME" request with the image names
    let request = format!("SEND_ME {}", image_names.join(","));
    socket.write_all(request.as_bytes()).await?;
    println!("'SEND_ME' request sent for images: {:?}", image_names);

    // Read and send the requested images
    for image_name in &image_names {
        let path = format!("./images/{}", image_name);
        if let Ok(mut file) = tokio::fs::File::open(&path).await {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;
            socket.write_all(&buffer).await?;
            println!("Image '{}' sent to the target client.", image_name);
        } else {
            eprintln!("Image '{}' not found in the 'images' folder.", image_name);
        }
    }

    Ok::<(), io::Error>(())
}

// Handles incoming "SEND_ME" requests
pub async fn handle_send_me_request(request: &str, mut socket: TcpStream) -> io::Result<()> {
    // Extract image names from the request (e.g., "SEND_ME img1.jpg,img2.png")
    let image_names: Vec<&str> = request
        .trim_start_matches("SEND_ME ")
        .split(',')
        .map(|name| name.trim())
        .collect();

    for image_name in image_names {
        let mut buffer = Vec::new();

        // Receive the image data
        let n = socket.read_to_end(&mut buffer).await?;

        // Save the received image to the "borrowedImages" folder
        let file_path = format!("./borrowedImages/{}", image_name);
        tokio::fs::create_dir_all("./borrowedImages").await?; // Ensure the folder exists
        tokio::fs::write(&file_path, &buffer[..n]).await?;

        println!(
            "Received image '{}' and saved to 'borrowedImages' folder.",
            image_name
        );
    }

    Ok::<(), io::Error>(())
}
