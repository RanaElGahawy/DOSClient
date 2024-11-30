use std::path::Path;
use image::{open, ImageBuffer, Rgba};
use crate::decoder::decode_and_display_image; // Import the decoding function

pub async fn view_image(image_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let image_path = format!("./borrowed_images/{}", image_name);

    // Check if the image exists in the "borrowed_images" folder
    if Path::new(&image_path).exists() {
        println!("Image found: {}", image_path);

        // Load the image and convert it to ImageBuffer
        let loaded_image = open(&image_path)?.to_rgba();

        // Decode and display the image
        decode_and_display_image(&loaded_image);
        println!("Image '{}' has been decoded and displayed.", image_name);
    } else {
        println!("Image '{}' not found in the 'borrowed_images' folder.", image_name);
    }

    Ok(())
}