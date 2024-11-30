use steganography::decoder;
use std::env;
use open;
use image::{ImageBuffer, Rgba, GenericImageView};
use std::fs::File;
use std::io::Write;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: decoder <encoded_image_name>");
        return;
    }
    let encoded_image_name = &args[1];

    // Adjust path to fetch images from `borrowed_images` folder
    let encoded_image_path = format!("./borrowed_images/{}", encoded_image_name);

    // Load the encoded image
    let mut encoded_image = image::open(&encoded_image_path)
        .expect("Failed to open encoded image")
        .to_rgba();

    // Decode access rights from the image
    let access_rights = get_access_rights_from_image(&encoded_image);

    if access_rights > 0 {
        println!("Access rights remaining: {}", access_rights);

        // Decrement access rights
        decrement_access_rights(&mut encoded_image, &encoded_image_path, access_rights);

        // Decode and display the hidden image
        decode_and_display_image(&encoded_image);
    } else {
        println!("No access rights remaining. Displaying the encoded image.");
        display_encoded_image(&encoded_image_path);
    }
}

pub fn decode_and_display_image(encoded_image: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
    // Create a decoder with the encoded image
    let decoder = decoder::Decoder::new(encoded_image.clone());
    let decoded_data = decoder.decode_alpha();

    // Save the decoded data to a temporary file
    let temp_file_path = "./borrowed_images/temp_decoded_image.png";
    let mut temp_file = File::create(temp_file_path).expect("Failed to create temporary file");
    temp_file
        .write_all(&decoded_data)
        .expect("Failed to write decoded data");

    // Open the decoded image using the system's default viewer
    open::that(temp_file_path).expect("Failed to open the decoded image");
}

fn get_access_rights_from_image(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> u32 {
    let (_, height) = image.dimensions();

    // Get the first pixel of the last row
    let pixel = image.get_pixel(0, height - 1);
    pixel[0] as u32 // Access rights stored in the red channel
}

fn decrement_access_rights(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    image_path: &str,
    access_rights: u32,
) {
    let (_, height) = image.dimensions();

    // Update the first pixel of the last row
    {
        let pixel = image.get_pixel_mut(0, height - 1);
        pixel[0] = (access_rights - 1) as u8; // Decrement access rights
    }

    // Save the updated image back to the original path
    image.save(image_path).expect("Failed to save updated image");
    println!(
        "Access rights decremented. New value: {}. Updated image saved.",
        access_rights - 1
    );
}

fn display_encoded_image(encoded_image_path: &str) {
    // Open the encoded image using the system's default viewer
    open::that(encoded_image_path).expect("Failed to open the encoded image using the default viewer");
}