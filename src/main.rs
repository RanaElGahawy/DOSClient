use std::env;
use std::fs;
use std::io::{self, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: client <server_ip:port>");
        return Ok(());
    }

    let server_addr = args[1].to_string(); // Create server address as a String

    println!("Starting client and listening at: {}", server_addr);

    // Clone the server_addr to avoid moving it into the spawn
    let server_addr_for_listener = server_addr.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_requests(&server_addr_for_listener).await {
            eprintln!("Error in listener: {}", e);
        }
    });

    loop {
        println!("Enter 1 to register, 2 to sign out, 3 to 'show me', 4 to 'send me':");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "1" => {
                match register_with_server(&server_addr).await {
                    Ok(client_id) => println!("Client registered with ID: {}", client_id),
                    Err(e) => eprintln!("Failed to register with server: {}", e),
                }
            }
            "2" => {
                loop {
                    match sign_out(&server_addr).await {
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
            "3" => {
                println!("Enter target client address (IP:port):");
                let mut target_addr = String::new();
                io::stdin().read_line(&mut target_addr)?;
                match show_me(target_addr.trim()).await {
                    Ok(image_list) => println!("Available images: {:?}", image_list),
                    Err(e) => eprintln!("Failed to fetch images: {}", e),
                }
            }
            "4" => {
                println!("Enter target client address (IP:port):");
                let mut target_addr = String::new();
                io::stdin().read_line(&mut target_addr)?;
                println!("Enter image names to send (comma-separated):");
                let mut image_names = String::new();
                io::stdin().read_line(&mut image_names)?;
                let image_names: Vec<&str> = image_names.trim().split(',').collect();
                if let Err(e) = send_me(target_addr.trim(), image_names).await {
                    eprintln!("Failed to send images: {}", e);
                }
            }
            _ => println!("Invalid input. Please enter 1, 2 or 3."),
        }
    }
}



// Registers with the server
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

// Signs out from the server
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

// Sends a "SHOW_ME" request to another client
async fn show_me(target_addr: &str) -> io::Result<Vec<String>> {
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(target_addr)).await??;

    println!("Connected to target client.");

    // Send "SHOW_ME" request
    timeout(Duration::from_secs(5), socket.write_all(b"SHOW_ME")).await??;
    println!("'SHOW_ME' request sent.");

    // Read the list of image names/IDs from the target client
    let mut buffer = [0u8; 1024]; // Buffer for the response
    let n = timeout(Duration::from_secs(5), socket.read(&mut buffer)).await??;

    let response = String::from_utf8_lossy(&buffer[..n]).to_string();
    let image_list: Vec<String> = response.split(',').map(|s| s.trim().to_string()).collect();

    Ok(image_list)
}

// Handles incoming "SHOW_ME" requests
async fn handle_show_me_request() -> io::Result<String> {
    // List files in the "images" folder
    let files = fs::read_dir("./images")?;
    let mut image_names = Vec::new();
    for file in files {
        if let Ok(entry) = file {
            if let Some(filename) = entry.file_name().to_str() {
                image_names.push(filename.to_string());
            }
        }
    }
    Ok(image_names.join(","))
}

async fn send_me(target_addr: &str, image_names: Vec<&str>) -> io::Result<()> {
    let mut socket = timeout(Duration::from_secs(5), TcpStream::connect(target_addr)).await??;

    println!("Connected to target client.");

    // Format the "SEND_ME" request with the image names
    let request = format!("SEND_ME {}", image_names.join(","));
    timeout(Duration::from_secs(5), socket.write_all(request.as_bytes())).await??;
    println!("'SEND_ME' request sent for images: {:?}", image_names);

    // Read and send the requested images
    for image_name in &image_names {
        let path = format!("./images/{}", image_name);
        if let Ok(mut file) = tokio::fs::File::open(&path).await {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;
            timeout(Duration::from_secs(10), socket.write_all(&buffer)).await??;
            println!("Image '{}' sent to the target client.", image_name);
        } else {
            eprintln!("Image '{}' not found in the 'images' folder.", image_name);
        }
    }

    Ok(())
}

async fn handle_send_me_request(request: &str, mut socket: TcpStream) -> io::Result<()> {
    // Extract image names from the request (e.g., "SEND_ME img1.jpg,img2.png")
    let image_names: Vec<&str> = request.trim_start_matches("SEND_ME ")
        .split(',')
        .map(|name| name.trim())
        .collect();

    for image_name in image_names {
        let mut buffer = Vec::new();

        // Receive the image data
        let n = timeout(Duration::from_secs(10), socket.read_to_end(&mut buffer)).await??;

        // Save the received image to the "borrowedImages" folder
        let file_path = format!("./borrowedImages/{}", image_name);
        tokio::fs::create_dir_all("./borrowedImages").await?; // Ensure the folder exists
        tokio::fs::write(&file_path, &buffer[..n]).await?;

        println!(
            "Received image '{}' and saved to 'borrowedImages' folder.",
            image_name
        );
    }

    Ok(())
}


async fn listen_for_requests(addr: &str) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Listening for requests on {}", addr);

    loop {
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            if let Ok(n) = socket.read(&mut buffer).await {
                let request = String::from_utf8_lossy(&buffer[..n]);
                if request.trim().starts_with("SHOW_ME") {
                    if let Ok(response) = handle_show_me_request().await {
                        let _ = socket.write_all(response.as_bytes()).await;
                    }
                } else if request.trim().starts_with("SEND_ME") {
                    if let Err(e) = handle_send_me_request(request.trim(), socket).await {
                        eprintln!("Error handling 'SEND_ME' request: {}", e);
                    }
                } else {
                    eprintln!("Unknown request: {}", request);
                }
            }
        });
    }
}
