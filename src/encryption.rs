use std::io::{self, Cursor};
use std::path::Path;
use image::{DynamicImage, ImageBuffer, ImageFormat, RgbaImage};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use steganography::encoder;
use std::sync::Arc;

// The main encryption function used by the server
#[allow(dead_code)]
pub async fn encode_and_send(
    file_name: String,
    encoded_file_name: String,
    mut socket: TcpStream,
    request_count: Arc<Mutex<u32>>,
) -> io::Result<()> {
    println!("Starting encoding for: {}", file_name);

    // Step 1: Encode the image
    let encoded_image_path = match encode_image_with_hidden(&file_name, 5) { // Default access rights = 5
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error encoding image {}: {:?}", file_name, e);
            return Err(io::Error::new(io::ErrorKind::Other, e));
        }
    };

    println!("Encoding completed. Sending back encoded image: {}", encoded_image_path);

    // Step 2: Send the encoded image back to the client
    let mut file = fs::File::open(&encoded_image_path).await?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;

    socket.write_all(&buffer).await?;
    println!("Sent encoded image: {}", encoded_file_name);

    // Decrement request_count when handling completes
    {
        let mut count = request_count.lock().await;
        *count -= 1;
        println!("Request count decremented to: {}", *count);
    }

    Ok(())
}

pub fn encode_image_with_hidden(
    hide_image_path: &str,
    access_rights: u8, // New parameter for access rights
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let hidden_image_path = Path::new(hide_image_path);
    if !hidden_image_path.exists() {
        return Err(format!("File not found: {}", hide_image_path).into());
    }

    let hidden_image = image::open(&hidden_image_path)?.to_rgba();
    let resized_hidden_image = image::imageops::resize(
        &hidden_image,
        hidden_image.width() / 2,
        hidden_image.height() / 2,
        image::imageops::FilterType::Lanczos3,
    );

    let mut hidden_image_bytes: Vec<u8> = Vec::new();
    DynamicImage::ImageRgba8(resized_hidden_image)
        .write_to(&mut Cursor::new(&mut hidden_image_bytes), ImageFormat::JPEG)?;

    let cover_image_path = Path::new("./default.png");
    if !cover_image_path.exists() {
        return Err("Cover image (default.png) not found.".into());
    }

    let default_image = image::open(cover_image_path)?.to_rgba();
    let resized_cover_image = image::imageops::resize(
        &default_image,
        hidden_image.width(),
        hidden_image.height(),
        image::imageops::FilterType::Lanczos3,
    );

    let encoder = encoder::Encoder::new(&hidden_image_bytes, DynamicImage::ImageRgba8(resized_cover_image));
    let mut encoded_image = encoder.encode_alpha();

    // Step: Add access rights to the last row of pixels
    add_access_rights_to_image(&mut encoded_image, access_rights)?;

    let encoded_image_path = format!("{}_encoded.png", hide_image_path);
    encoded_image.save(&encoded_image_path)?;

    Ok(encoded_image_path)
}

// Function to add access rights to the last row of pixels
fn add_access_rights_to_image(
    image: &mut ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    access_rights: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (width, height) = image.dimensions();

    // Ensure the image has at least one row
    if height == 0 {
        return Err("Image height is zero.".into());
    }

    // Write the access rights number into the last row of pixels
    for x in 0..width {
        let pixel = image.get_pixel_mut(x, height - 1);
        pixel[0] = access_rights; // Store the access rights in the red channel
        pixel[1] = 0; // Clear other channels (optional)
        pixel[2] = 0;
        pixel[3] = 255; // Ensure alpha remains 255
    }

    Ok(())
}

// New main function for standalone testing
#[tokio::main]
async fn main() -> io::Result<()> {
    // Prompt the user for the file name and access rights
    println!("Enter the path to the image to hide:");
    let mut hide_image_path = String::new();
    io::stdin().read_line(&mut hide_image_path)?;
    let hide_image_path = hide_image_path.trim();

    println!("Enter the number of access rights to encode:");
    let mut access_rights_input = String::new();
    io::stdin().read_line(&mut access_rights_input)?;
    let access_rights: u8 = access_rights_input.trim().parse().unwrap_or(5); // Default to 5 if invalid input

    // Call the encoding function
    match encode_image_with_hidden(hide_image_path, access_rights) {
        Ok(encoded_image_path) => {
            println!("Image successfully encoded and saved at: {}", encoded_image_path);
        }
        Err(e) => {
            eprintln!("Error encoding image: {:?}", e);
        }
    }

    Ok(())
}
/// Updates access rights in the metadata without re-encrypting the entire image.
pub fn update_access_rights(image_path: &str, new_access_rights: u8) -> io::Result<()> {
    let mut image = image::open(image_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        .to_rgba();

    let (_, height) = image.dimensions();
    let pixel = image.get_pixel_mut(0, height - 1);
    pixel[0] = new_access_rights; // Update the red channel with the new access rights

    image.save(image_path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?; // Save updated image

    println!("Updated access rights in '{}'. New value: {}", image_path, new_access_rights);

    Ok(())
}