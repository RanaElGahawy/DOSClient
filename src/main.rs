mod show_me;
mod send_me;
mod view;
mod encryption;
mod decoder;

use show_me::{handle_show_me_request, send_show_me_request};
use send_me::{send_me_request, handle_send_me_request};
use view::view_image;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: client <server_ip:port>");
        return Ok(());
    }

    let server_addr = args[1].to_string();

    println!("Starting client and listening at: {}", server_addr);

    tokio::spawn(async move {
        if let Err(e) = listen_for_requests(&server_addr).await {
            eprintln!("Error in listener: {}", e);
        }
    });

    loop {
        println!("Enter 1 to register, 2 to sign out, 3 to 'show me', 4 to 'send me', 5 to 'view':");
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
                println!("Enter the image names to send (this option should not be used, send requests in '3'): ");
            }
            "5" => {
                println!("Enter the image name to view:");
                let mut image_name = String::new();
                io::stdin().read_line(&mut image_name)?;
                let image_name = image_name.trim();
                if let Err(e) = view_image(image_name).await {
                    eprintln!("Failed to view image: {}", e);
                }
            }
            _ => println!("Invalid input."),
        }
    }
}

// Handles incoming requests
async fn listen_for_requests(addr: &str) -> io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Listening for requests on {}", addr);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let mut buffer = [0u8; 1024];
        let n = socket.read(&mut buffer).await?;
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
}