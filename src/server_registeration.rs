use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

pub async fn register_with_server(server_addrs: &Vec<&str>) -> io::Result<String> {
    for server_addr in server_addrs {
        match timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await {
            Ok(Ok(mut socket)) => {
                println!("Connected to server at {}.", server_addr);

                // Send registration request
                if let Err(e) = timeout(Duration::from_secs(5), socket.write_all(b"JOIN")).await {
                    eprintln!("Failed to send registration request to {}: {}", server_addr, e);
                    continue; // Try the next server
                }
                println!("Registration request sent to {}.", server_addr);

                // Read the assigned unique client ID from the server
                let mut buffer = [0u8; 128];
                match timeout(Duration::from_secs(5), socket.read(&mut buffer)).await {
                    Ok(Ok(n)) => {
                        let client_id = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("Received client ID from {}: {}", server_addr, client_id);
                        return Ok(client_id);
                    }
                    Ok(Err(e)) => eprintln!("Failed to read response from {}: {}", server_addr, e),
                    Err(_) => eprintln!("Timeout while reading response from {}.", server_addr),
                }
            }
            Ok(Err(_)) => {},
            Err(_) => {},
        }
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to connect to any server",
    ))
}


pub async fn rejoin_with_server(server_addrs: &Vec<&str>, client_id: &str) -> io::Result<String> {
    for server_addr in server_addrs {
        match timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await {
            Ok(Ok(mut socket)) => {
                println!("Connected to server at {}.", server_addr);

                // Send rejoin request
                let rejoin_message = format!("REJOIN {}", client_id);
                if let Err(e) = timeout(Duration::from_secs(5), socket.write_all(rejoin_message.as_bytes())).await {
                    eprintln!("Failed to send rejoin request to {}: {}", server_addr, e);
                    continue; // Try the next server
                }
                println!("Rejoin request sent to {} with ID: {}", server_addr, client_id);

                // Read the server's response
                let mut buffer = [0u8; 128];
                match timeout(Duration::from_secs(5), socket.read(&mut buffer)).await {
                    Ok(Ok(n)) => {
                        let response = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("Rejoin response from {}: {}", server_addr, response);
                        return Ok(response); // Successfully rejoined
                    }
                    Ok(Err(e)) => eprintln!("Failed to read response from {}: {}", server_addr, e),
                    Err(_) => eprintln!("Timeout while reading response from {}.", server_addr),
                }
            }
            Ok(Err(_)) => {},
            Err(_) => {},
        }
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to reconnect with any server",
    ))
}


pub async fn sign_out(servers: &Vec<&str>, client_id: &str) -> io::Result<String> {
    for server_addr in servers {
        match timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await {
            Ok(Ok(mut socket)) => {
                println!("Connected to server at {}.", server_addr);

                // Send sign-out request with client ID
                let sign_out_message = format!("SIGN_OUT {}", client_id);
                if let Err(e) = timeout(Duration::from_secs(5), socket.write_all(sign_out_message.as_bytes())).await {
                    eprintln!("Failed to send sign-out request to {}: {}", server_addr, e);
                    continue; // Try the next server
                }
                println!("Sign out request sent to {} with ID: {}", server_addr, client_id);

                // Read the acknowledgment from the server
                let mut buffer = [0u8; 128];
                match timeout(Duration::from_secs(5), socket.read(&mut buffer)).await {
                    Ok(Ok(n)) => {
                        let ack = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("Sign out status from {}: {}", server_addr, ack);
                        return Ok(ack); // Return acknowledgment if successful
                    }
                    Ok(Err(e)) => eprintln!("Failed to read acknowledgment from {}: {}", server_addr, e),
                    Err(_) => eprintln!("Timeout while reading acknowledgment from {}.", server_addr),
                }
            }
            Ok(Err(_)) => {},
            Err(_) => {},
        }
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to sign out with any server",
    ))
}


pub async fn mark_client_unreachable(servers: &Vec<&str>, client_id: &str) -> io::Result<()> {
    for server_addr in servers {
        match timeout(Duration::from_secs(5), TcpStream::connect(server_addr)).await {
            Ok(Ok(mut socket)) => {
                println!("Connected to server at {}.", server_addr);

                // Send the "UNREACHABLE" message to the server
                let unreachable_message = format!("UNREACHABLE {}", client_id);
                if let Err(e) = timeout(Duration::from_secs(5), socket.write_all(unreachable_message.as_bytes())).await {
                    eprintln!("Failed to send unreachable request to {}: {}", server_addr, e);
                    continue; // Try the next server
                }
                println!("Unreachable request sent to {} with ID: {}", server_addr, client_id);

                // No need to read the response, simply return success
                return Ok(());
            }
            Ok(Err(_)) => {},
            Err(_) => {},
        }
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to mark client as unreachable with any server",
    ))
}
