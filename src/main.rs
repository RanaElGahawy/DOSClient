use std::fs::File;
use std::io::{self, Read, Write};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::UdpSocket;
use tokio::task;

mod server_registeration;
mod active_clients; // Include the new module
mod encryption;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <self_ip:port> <next_ip:port> <prev_ip:port>", args[0]);
        return Ok(());
    }

    // Use references to avoid ownership issues
    let servers: Vec<&str> = vec![&args[1], &args[2], &args[3]];

    let mut client_id = String::new();
    let active_clients = Arc::new(Mutex::new(HashMap::new())); // Shared active clients list

    // Check if client_ID file exists
    if let Ok(mut file) = File::open("client_ID") {
        let mut id = String::new();
        file.read_to_string(&mut id)?;
        client_id = id.trim().to_string();
        println!("Found existing client ID: {}", client_id);

        // Send REJOIN request
        match server_registeration::rejoin_with_server(&servers, &client_id).await {
            Ok(response) => println!("Rejoin successful: {}", response),
            Err(e) => eprintln!("Failed to rejoin with server: {}", e),
        }
    } else {
        // No client_ID file, register with the server
        println!("No existing client ID found. Registering with the server...");
        match server_registeration::register_with_server(&servers).await {
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
                    match server_registeration::sign_out(&servers, &client_id).await {
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
                match active_clients::show_active_clients(&servers, Arc::clone(&active_clients)).await {
                    Ok(_) => {
                        let clients = active_clients.lock().await; // Asynchronously acquire the lock
                        println!("Active clients: {:?}", *clients);
                    }
                    Err(e) => eprintln!("Failed to fetch active clients: {}", e),
                }
            }
            "2" => {
                println!("Enter the ID of the client to mark as unreachable:");
                let mut unreachable_id = String::new();
                io::stdin().read_line(&mut unreachable_id)?;
                let unreachable_id = unreachable_id.trim();

                if unreachable_id.is_empty() {
                    eprintln!("Client ID cannot be empty.");
                    continue;
                }

                match server_registeration::mark_client_unreachable(&servers, unreachable_id).await {
                    Ok(_) => println!("Successfully marked client ID {} as unreachable", unreachable_id),
                    Err(e) => eprintln!("Failed to mark client ID {} as unreachable: {}", unreachable_id, e),
                }
            }
            "3" => {
                println!("Enter the path to the image file you want to send:");
                let mut image_path = String::new();
                io::stdin().read_line(&mut image_path)?;
                let image_path = image_path.trim();

                let save_folder = "Borrowed Images";
                let timeout_duration = std::time::Duration::from_secs(60);

                let mut tasks = Vec::new();

                for server in &servers { // Use a reference to iterate over the servers
                    let server = server.to_string(); // Clone for each task
                    let image_path = image_path.to_string();
                    let save_folder = save_folder.to_string();

                    tasks.push(tokio::spawn(async move {
                        encryption::perform_image_encryption(&server, &image_path, &save_folder, timeout_duration).await
                    }));
                }

                for task in tasks {
                    match task.await {
                        Ok(Ok(())) => {
                            println!("Encryption process completed successfully by one of the servers.");
                            break;
                        }
                        Ok(Err(_)) => {}
                        Err(_) => {}
                    }
                }
            }
            _ => println!("Invalid input. Please enter 0 to sign out or 1 to show active clients."),
        }
    }
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
