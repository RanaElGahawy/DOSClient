use tokio::net::TcpStream;
use tokio::io::{self, AsyncWriteExt};

pub async fn send_encryption_request(server_addr: &str) -> io::Result<()> {
    let mut socket = TcpStream::connect(server_addr).await?;
    let encryption_message = "ENCRYPTION";

    socket.write_all(encryption_message.as_bytes()).await?;
    println!("Sent ENCRYPTION request to server.");
    Ok(())
}

