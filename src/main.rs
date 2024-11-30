use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

mod server_registeration;
mod active_clients; // Include the new module

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: client <server_ip:port>");
        return Ok(());
    }

    let server_addr = &args[1];

    if !server_addr.contains(':') {
        eprintln!("Invalid server address. Expected format: <ip>:<port>");
        return Ok(());
    }

    let mut client_id = String::new();
    let active_clients = Arc::new(Mutex::new(HashMap::new())); // Shared active clients list

    // Check if client_ID file exists
    if let Ok(mut file) = File::open("client_ID") {
        let mut id = String::new();
        file.read_to_string(&mut id)?;
        client_id = id.trim().to_string();
        println!("Found existing client ID: {}", client_id);

        // Send REJOIN request
        match server_registeration::rejoin_with_server(server_addr, &client_id).await {
            Ok(response) => println!("Rejoin successful: {}", response),
            Err(e) => eprintln!("Failed to rejoin with server: {}", e),
        }
    } else {
        // No client_ID file, register with the server
        println!("No existing client ID found. Registering with the server...");
        match server_registeration::register_with_server(server_addr).await {
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

    loop {
        println!("Enter 0 to sign out, 1 to show active clients:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "0" => {
                if client_id.is_empty() {
                    println!("You must register first before signing out.");
                } else {
                    match server_registeration::sign_out(server_addr, &client_id).await {
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
                match active_clients::show_active_clients(server_addr, Arc::clone(&active_clients)).await {
                    Ok(_) => {
                        let clients = active_clients.lock().await; // Asynchronously acquire the lock
                        println!("Active clients: {:?}", *clients);
                    }
                    Err(e) => eprintln!("Failed to fetch active clients: {}", e),
                }
            }
            _ => println!("Invalid input. Please enter 0 to sign out or 1 to show active clients."),
        }
    }
}

fn save_client_id_to_file(client_id: &str) -> io::Result<()> {
    let mut file = File::create("client_ID")?;
    file.write_all(client_id.as_bytes())?;
    Ok(())
}
