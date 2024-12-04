mod show_me;
mod send_me;
mod view;
mod encryption;
mod decoder;
mod server_registeration;
mod active_clients;

use show_me::{handle_show_me_request, send_show_me_request};
use send_me::{send_me_request, handle_send_me_request_with_prompt};
use std::fs::File;
use view::view_image; // Function for option 5
use crate::encryption::update_access_rights;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::io::{self, Read, Write};
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

    let client_addr = "10.40.43.42:8080".to_string();

    let mut client_id = String::new();
    let active_clients: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new())); // Shared active clients list
    let access_rights_state = Arc::new(Mutex::new(AccessRightsState::default()));

    // Check if client_ID file exists
    if let Ok(mut file) = File::open("client_ID") {
        let mut id = String::new();
        file.read_to_string(&mut id)?;
        client_id = id.trim().to_string();
        println!("Found existing client ID: {}", client_id);

        // Send REJOIN request
        match server_registeration::rejoin_with_server(&server_addr, &client_id).await {
            Ok(response) => println!("Rejoin successful: {}", response),
            Err(e) => eprintln!("Failed to rejoin with server: {}", e),
        }
    } else {
        // No client_ID file, register with the server
        println!("No existing client ID found. Registering with the server...");
        match server_registeration::register_with_server(&server_addr).await {
            Ok(id) => {
                client_id = id.clone();
                if let Err(e) = save_client_id_to_file(&client_id) {
                    eprintln!("Failed to save client ID to file: {}", e);
                }
                println!("Client registered with ID: {}", client_id);
            }
            Err(e) => eprintln!("Failed to register with server: {}", e),
        }
    }

    task::spawn({
        let access_rights_state_clone = Arc::clone(&access_rights_state);
        let server_ip_and_port = server_addr.clone();
        let active_clients_clone = Arc::clone(&active_clients);
        async move {
            if let Err(e) = listen_for_requests(
                &client_addr,
                access_rights_state_clone,
                &server_ip_and_port,
                &client_id,
                active_clients_clone,
            )
            .await
            {
                eprintln!("Error in listener: {}", e);
            }
        }
    });

    loop {
        println!(
            "Enter:\n\
             1 to show active clients\n\
             2 to mark a client as unreachable\n\
             3 to 'send me'\n\
             4 to 'show me'\n\
             5 to 'view'\n\
             AR for Access Rights\n\
             6 to update access rights"
        );
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => {
                match active_clients::show_active_clients(&server_addr, Arc::clone(&active_clients)).await {
                    Ok(_) => {
                        let clients = active_clients.lock().await;
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

                match server_registeration::mark_client_unreachable(&server_addr, unreachable_id).await {
                    Ok(_) => println!("Successfully marked client ID {} as unreachable", unreachable_id),
                    Err(e) => eprintln!("Failed to mark client ID {} as unreachable: {}", unreachable_id, e),
                }
            }
            "3" => {
                println!("Enter the ID:port of the client to request images from:");
                let mut client_id_port = String::new();
                io::stdin().read_line(&mut client_id_port)?;
                let client_id_port = client_id_port.trim();

                if let Some(target_addr) = get_target_address(client_id_port, Arc::clone(&active_clients)).await {
                    println!("Enter image names to request (comma-separated):");
                    let mut image_names = String::new();
                    io::stdin().read_line(&mut image_names)?;
                    let image_names: Vec<&str> = image_names.trim().split(',').collect();

                    if let Err(e) = send_me_request(&target_addr, image_names).await {
                        eprintln!("Failed to request images: {}", e);
                        if e.kind() == io::ErrorKind::ConnectionRefused {
                            if let Err(err) = notify_server_of_unreachable_client(&server_addr, client_id_port).await {
                                eprintln!("Failed to notify server about unreachable client: {}", err);
                            }
                        }
                    }
                }
            }
            "4" => {
                println!("Enter the ID:port of the client to view images from:");
                let mut client_id_port = String::new();
                io::stdin().read_line(&mut client_id_port)?;
                let client_id_port = client_id_port.trim();

                if let Some(target_addr) = get_target_address(client_id_port, Arc::clone(&active_clients)).await {
                    match send_show_me_request(&target_addr).await {
                        Ok(image_list) => {
                            println!("Images available on the target client:");
                            for image in image_list {
                                println!("- {}", image);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to retrieve images: {}", e);
                            if e.kind() == io::ErrorKind::ConnectionRefused {
                                if let Err(err) = notify_server_of_unreachable_client(&server_addr, client_id_port).await {
                                    eprintln!("Failed to notify server about unreachable client: {}", err);
                                }
                            }
                        }
                    }
                }
            }
            "5" => {
                println!("Enter the name of the encoded image to view:");
                let mut image_name = String::new();
                io::stdin().read_line(&mut image_name)?;
                view_image(image_name.trim());
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
    println!("Enter the ID:port of the target client:");
    let mut client_id_port = String::new();
    io::stdin().read_line(&mut client_id_port)?;
    let client_id_port = client_id_port.trim();

    println!("Enter the name of the image to update:");
    let mut image_name = String::new();
    io::stdin().read_line(&mut image_name)?;
    let image_name = image_name.trim();

    println!("Enter the new access rights (1-5):");
    let mut new_access_rights = String::new();
    io::stdin().read_line(&mut new_access_rights)?;
    let new_access_rights: u8 = new_access_rights.trim().parse().unwrap_or(0);

    if new_access_rights < 1 || new_access_rights > 5 {
        println!("Invalid access rights. Please enter a value between 1 and 5.");
        continue;
    }

    if let Some(target_addr) = get_target_address(client_id_port, Arc::clone(&active_clients)).await {
        if let Err(e) = send_update_request(&target_addr, image_name, new_access_rights).await {
            eprintln!("Failed to send update request: {}", e);
            if e.kind() == io::ErrorKind::ConnectionRefused {
                // Notify server about unreachable client
                let parts: Vec<&str> = client_id_port.split(':').collect();
                if parts.len() == 2 {
                    let client_id = parts[0].trim();
                    if let Err(err) = notify_server_of_update_failure(
                        &server_addr,
                        client_id,
                        image_name,
                        new_access_rights,
                    )
                    .await
                    {
                        eprintln!("Failed to notify server about unreachable client: {}", err);
                    }
                }
            }
        }
    } else {
        eprintln!("Could not resolve target address for client '{}'.", client_id_port);
    }
}
            _ => println!("Invalid input."),
        }
    }
}

async fn notify_server_of_unreachable_client(
    server_ip_and_port: &str,
    client_id_port: &str,
) -> io::Result<()> {
    let parts: Vec<&str> = client_id_port.split(':').collect();
    if parts.len() != 2 {
        eprintln!("Invalid client_id:port format: {}", client_id_port);
        return Ok(());
    }
    let client_id = parts[0].trim();

    let mut socket = TcpStream::connect(server_ip_and_port).await?;
    let message = format!("UNREACHABLE {}", client_id);
    socket.write_all(message.as_bytes()).await?;
    println!("Notified server about unreachable client ID: {}", client_id);
    Ok(())
}

async fn get_target_address(
    client_id_port: &str,
    active_clients: Arc<Mutex<HashMap<String, String>>>,
) -> Option<String> {
    let parts: Vec<&str> = client_id_port.split(':').collect();
    if parts.len() != 2 {
        eprintln!("Invalid client_id:port format: {}", client_id_port);
        return None;
    }
    let client_id = parts[0].trim();
    let port = parts[1].trim();

    let clients = active_clients.lock().await;
    if let Some(ip) = clients.get(client_id) {
        Some(format!("{}:{}", ip, port))
    } else {
        eprintln!("Client ID '{}' not found in the active clients map.", client_id);
        None
    }
}

fn save_client_id_to_file(client_id: &str) -> io::Result<()> {
    let mut file = File::create("client_ID")?;
    file.write_all(client_id.as_bytes())?;
    Ok(())
}


async fn listen_for_requests(
    addr: &str,
    access_rights_state: Arc<Mutex<AccessRightsState>>,
    server_ip_and_port: &str,
    client_id: &str,
    active_clients: Arc<Mutex<HashMap<String, String>>>,
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
            let mut locked_socket = socket.lock().await;
            let tcp_stream = &mut *locked_socket;

            process_update_request(request.trim(), tcp_stream).await?;
        } else {
            eprintln!("Unknown request: {}", request);
        }
    }
}

async fn process_update_request(
    request: &str,
    socket: &mut TcpStream,
) -> io::Result<()> {
    let parts: Vec<&str> = request.split_whitespace().collect();
    if parts.len() != 3 {
        eprintln!("Invalid UPDATE request: {}", request);
        return Ok(());
    }

    let image_name = parts[1];
    let new_access_rights: u8 = parts[2].parse().unwrap_or(0);

    if new_access_rights < 1 || new_access_rights > 5 {
        eprintln!("Invalid access rights in UPDATE request: {}", request);
        return Ok(());
    }

    let image_path = format!("./borrowed_images/{}", image_name);
    if !Path::new(&image_path).exists() {
        eprintln!("Image '{}' not found in 'borrowed_images'.", image_name);
        return Ok(());
    }

    // Update the image's access rights
    update_access_rights(&image_path, new_access_rights)?;
    println!("Access rights for '{}' updated to '{}'.", image_name, new_access_rights);

    // Respond to originating client
    socket
        .write_all(b"Access rights updated successfully.\n")
        .await?;

    Ok(())
}

async fn notify_server_of_update_failure(
    server_ip_and_port: &str,
    client_id: &str,
    image_name: &str,
    new_access_rights: u8,
) -> io::Result<()> {
    let mut socket = TcpStream::connect(server_ip_and_port).await?;
    let message = format!("UPDATE {} {} {}", client_id, image_name, new_access_rights);
    socket.write_all(message.as_bytes()).await?;
    println!("Notified server about UPDATE failure: {}", message);
    Ok(())
}

async fn send_update_request(target_addr: &str, image_name: &str, new_access_rights: u8) -> io::Result<()> {
    let mut socket = TcpStream::connect(target_addr).await?;
    println!("Connected to target client.");

    let request = format!("UPDATE {} {}", image_name, new_access_rights);
    socket.write_all(request.as_bytes()).await?;
    println!("Update request sent for '{}' with new access rights: {}", image_name, new_access_rights);

    Ok(())
}


async fn send_image_with_rights(
    image_name: String,
    rights: u8,
    socket: Arc<Mutex<TcpStream>>,
) -> io::Result<()> {
    let image_path = format!("./images/{}", image_name);
    
    // Check if the image exists
    if Path::new(&image_path).exists() {
        // Encrypt or encode the image with hidden access rights
        let encoded_image_path = encode_image_with_hidden(&image_path, rights).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Error encoding image: {:?}", e))
        })?;

        // Open the encoded image file
        let mut file = tokio::fs::File::open(&encoded_image_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        // Send the file to the requesting client via the provided socket
        let mut locked_socket = socket.lock().await; // Acquire the lock for the socket
        locked_socket.write_all(&buffer).await?;
        println!(
            "Image '{}' sent successfully with rights '{}'.",
            image_name, rights
        );
    } else {
        println!("Image '{}' not found in the images folder.", image_name);
    }

    Ok(())
}