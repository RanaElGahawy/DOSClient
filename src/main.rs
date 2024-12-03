mod show_me;
mod send_me;
mod view;
mod encryption;
mod decoder;

use show_me::{handle_show_me_request, send_show_me_request};
use send_me::{send_me_request, handle_send_me_request_with_prompt};
use view::view_image; // Function for option 5

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::io;
use tokio::task;
use std::path::Path;
use crate::encryption::encode_image_with_hidden;

#[derive(Default)]
struct AccessRightsState {
    pending_request: Option<String>, // Holds the image name waiting for access rights
    socket: Option<Arc<Mutex<TcpStream>>>, // Arc-wrapped socket for thread-safe sharing
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: client <server_ip:port>");
        return Ok(());
    }

    let server_addr = args[1].to_string();
    println!("Starting client and listening at: {}", server_addr);

    let access_rights_state = Arc::new(Mutex::new(AccessRightsState::default()));
    let server_addr_clone = server_addr.clone();
    let access_rights_state_clone = Arc::clone(&access_rights_state);

    task::spawn(async move {
        if let Err(e) = listen_for_requests(&server_addr_clone, access_rights_state_clone).await {
            eprintln!("Error in listener: {}", e);
        }
    });

    loop {
        println!("Enter 1 to register  \n 2 to sign out\n 3 to 'send me'\n 4 to 'show me'\n 5 to 'view'\nAR for Access Rights\n6 to update access rights:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "3" => {
                println!("Enter target client address (IP:port):");
                let mut target_addr = String::new();
                io::stdin().read_line(&mut target_addr)?;

                println!("Enter image names to request (comma-separated):");
                let mut image_names = String::new();
                io::stdin().read_line(&mut image_names)?;
                let image_names: Vec<&str> = image_names.trim().split(',').collect();

                if let Err(e) = send_me_request(target_addr.trim(), image_names).await {
                    eprintln!("Failed to request images: {}", e);
                }
            }
            "4" => {
                println!("Enter target client address (IP:port):");
                let mut target_addr = String::new();
                io::stdin().read_line(&mut target_addr)?;
                match send_show_me_request(target_addr.trim()).await {
                    Ok(image_list) => {
                        println!("Images available on the target client:");
                        for image in image_list {
                            println!("- {}", image);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to retrieve images: {}", e);
                    }
                }
            }
            "5" => {
                println!("Enter the name of the encoded image to view (from 'borrowed_images'):");
                let mut image_name = String::new();
                io::stdin().read_line(&mut image_name)?;
                let image_name = image_name.trim();

                view_image(image_name); // Call the view module's function
            }
            "AR" => {
                let mut state = access_rights_state.lock().await;
                if let Some(image_name) = state.pending_request.take() {
                    println!("Enter access rights for image '{}':", image_name);
                    let mut access_rights = String::new();
                    io::stdin().read_line(&mut access_rights)?;

                    match access_rights.trim().parse::<u8>() {
                        Ok(rights) if rights >= 1 && rights <= 5 => {
                            println!("Access rights '{}' accepted.", rights);
                            if let Some(socket) = state.socket.take() {
                                tokio::spawn(async move {
                                    if let Err(e) = send_image_with_rights(image_name, rights, socket).await {
                                        eprintln!("Error sending image: {}", e);
                                    }
                                });
                            }
                        }
                        _ => println!("Invalid access rights. Please enter a value between 1 and 5."),
                    }
                } else {
                    println!("No pending access rights requests.");
                }
            }
            "6" => {
                println!("Enter target client address (IP:port):");
                let mut target_addr = String::new();
                io::stdin().read_line(&mut target_addr)?;

                println!("Enter the name of the image to update:");
                let mut image_name = String::new();
                io::stdin().read_line(&mut image_name)?;

                println!("Enter the new access rights (0-10):");
                let mut new_access_rights = String::new();
                io::stdin().read_line(&mut new_access_rights)?;
                let new_access_rights: u8 = new_access_rights.trim().parse().unwrap_or(0);

                if new_access_rights < 1 || new_access_rights > 5 {
                    println!("Invalid access rights. Please enter a value between 1 and 5.");
                    continue;
                }

                if let Err(e) = send_update_request(
                    target_addr.trim(),
                    image_name.trim(),
                    new_access_rights,
                )
                .await
                {
                    eprintln!("Failed to send update request: {}", e);
                }
            }
            _ => println!("Invalid input."),
        }
    }
}

async fn listen_for_requests(
    addr: &str,
    access_rights_state: Arc<Mutex<AccessRightsState>>,
) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Listening for requests on {}", addr);

    loop {
        let socket = listener.accept().await?.0; // Get TcpStream directly
        let socket = Arc::new(Mutex::new(socket)); // Wrap in Arc<Mutex>

        let mut buffer = [0u8; 1024];
        let n = socket.lock().await.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..n]);

        if request.trim().starts_with("SHOW_ME") {
            if let Ok(response) = handle_show_me_request().await {
                let mut locked_socket = socket.lock().await;
                locked_socket.write_all(response.as_bytes()).await?;
            }
        } else if request.trim().starts_with("SEND_ME") {
            let image_names: Vec<&str> = request
                .trim_start_matches("SEND_ME ")
                .split(',')
                .map(|name| name.trim())
                .collect();

            for image_name in image_names {
                let mut state = access_rights_state.lock().await;
                state.pending_request = Some(image_name.to_string());
                state.socket = Some(Arc::clone(&socket));

                println!(
                    "Request received for image '{}'. Go to main menu and select 'AR' to provide access rights.",
                    image_name
                );
            }
        } else if request.trim().starts_with("UPDATE") {
            // Unlock `Arc<Mutex<TcpStream>>` to retrieve the inner `TcpStream`
            let mut locked_socket = socket.lock().await;
            let tcp_stream = &mut *locked_socket; // Extract the TcpStream
            process_update_request(request.trim(), tcp_stream).await?;
        } else {
            eprintln!("Unknown request: {}", request);
        }
    }
}

async fn send_update_request(target_addr: &str, image_name: &str, new_access_rights: u8) -> io::Result<()> {
    let mut socket = TcpStream::connect(target_addr).await?;
    println!("Connected to target client.");

    let request = format!("UPDATE {} {}", image_name, new_access_rights);
    socket.write_all(request.as_bytes()).await?;
    println!("Update request sent for '{}' with new access rights: {}", image_name, new_access_rights);

    Ok(())
}

async fn process_update_request(request: &str, socket: &mut TcpStream) -> io::Result<()> {
    let parts: Vec<&str> = request.split_whitespace().collect();
    if parts.len() != 3 {
        eprintln!("Invalid UPDATE request: {}", request);
        return Ok(());
    }

    let image_name = parts[1];
    let new_access_rights: u8 = parts[2].parse().unwrap_or(0);

    if new_access_rights < 0 || new_access_rights > 10 {
        eprintln!("Invalid access rights in UPDATE request: {}", request);
        return Ok(());
    }

    let image_path = format!("./borrowed_images/{}", image_name);
    if !Path::new(&image_path).exists() {
        eprintln!("Image '{}' not found in 'borrowed_images'.", image_name);
        return Ok(());
    }

    // Use the optimized function to update access rights
    update_access_rights(&image_path, new_access_rights)?;
    println!("Access rights for '{}' updated to '{}'.", image_name, new_access_rights);

    socket
        .write_all(b"Access rights updated successfully.\n")
        .await?;

    Ok(())
}

fn update_access_rights(image_path: &str, new_access_rights: u8) -> io::Result<()> {
    let mut image = image::open(image_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        .to_rgba();

    let (_, height) = image.dimensions();
    let pixel = image.get_pixel_mut(0, height - 1);
    pixel[0] = new_access_rights; // Update the red channel with the new access rights

    image.save(image_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?; // Save updated image

    Ok(())
}


async fn send_image_with_rights(image_name: String, rights: u8, socket: Arc<Mutex<TcpStream>>) -> io::Result<()> {
    let image_path = format!("./images/{}", image_name);
    if Path::new(&image_path).exists() {
        let encoded_image_path = encode_image_with_hidden(&image_path, rights).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Error encrypting image: {:?}", e))
        })?;

        let mut file = tokio::fs::File::open(&encoded_image_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let mut locked_socket = socket.lock().await; // Async lock
        locked_socket.write_all(&buffer).await?;

        println!("Image '{}' sent with rights '{}'.", image_name, rights);
    } else {
        println!("Image '{}' not found.", image_name);
    }
    Ok(())
}