use steganography::decoder;
use image::{ImageBuffer, Rgba, GenericImageView};
use std::process::Command;
use std::fs;

/// Handles viewing and decoding an image.
pub fn view_image(encoded_image_name: &str) {
    let encoded_image_path = format!("./borrowed_images/{}", encoded_image_name);

    // Load the encoded image
    let mut encoded_image = match image::open(&encoded_image_path) {
        Ok(img) => img.to_rgba(),
        Err(_) => {
            eprintln!("Failed to open encoded image '{}'", encoded_image_name);
            return;
        }
    };

    // Decode access rights from the image
    let access_rights = get_access_rights_from_image(&encoded_image);

    if access_rights > 0 {
        println!("Access rights remaining: {}", access_rights);

        // Decrement access rights
        decrement_access_rights(&mut encoded_image, &encoded_image_path, access_rights);

        println!("Access rights decremented. New value: {}", access_rights - 1);

        // Decode and display the hidden image
        decode_and_display_image(&encoded_image);
    } else {
        println!("No access rights remaining. Displaying the encoded image.");
        display_encoded_image(&encoded_image_path);
    }
}

/// Extracts access rights from the image's metadata.
fn get_access_rights_from_image(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> u32 {
    let (_, height) = image.dimensions();
    let pixel = image.get_pixel(0, height - 1);
    pixel[0] as u32 // Access rights stored in the red channel
}

/// Decrements access rights and updates the image.
fn decrement_access_rights(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    image_path: &str,
    access_rights: u32,
) {
    let (_, height) = image.dimensions();

    {
        let pixel = image.get_pixel_mut(0, height - 1);
        pixel[0] = (access_rights - 1) as u8; // Decrement access rights
    }

    if let Err(err) = image.save(image_path) {
        eprintln!("Failed to save updated image '{}': {}", image_path, err);
    }
}

/// Decodes and displays the hidden image.
fn decode_and_display_image(encoded_image: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let decoder = decoder::Decoder::new(encoded_image.clone());
    let decoded_data = decoder.decode_alpha();

    if let Ok(decoded_image) = image::load_from_memory(&decoded_data) {
        println!("Decoded image successfully. Now displaying...");

        let temp_file_path = "./borrowed_images/temp_decoded_image.png";
        decoded_image
            .save(temp_file_path)
            .expect("Failed to save temporary decoded image");

        open_image_and_wait(temp_file_path);

        if fs::remove_file(temp_file_path).is_ok() {
            println!("Temporary file deleted.");
        }
    } else {
        eprintln!("Failed to decode the image.");
    }
}

/// Displays the encoded image using the system's default viewer.
fn display_encoded_image(encoded_image_path: &str) {
    open_image_and_wait(encoded_image_path);
}

/// Opens an image using the system's default viewer and waits for it to close.
fn open_image_and_wait(file_path: &str) {
    #[cfg(target_os = "macos")]
    {
        Command::new("qlmanage")
            .args(&["-p", file_path])
            .status()
            .expect("Failed to open the image.");
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(&["/C", "start", "/WAIT", file_path])
            .status()
            .expect("Failed to open the image.");
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("eog")
            .arg(file_path)
            .status()
            .expect("Failed to open the image.");
    }
}