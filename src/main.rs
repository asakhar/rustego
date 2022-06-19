use std::{
  error::Error,
  io::{Read, Write},
  path::Path,
};

use stego_image::StegoImage;

mod stego_image;

fn main() -> Result<(), Box<dyn Error>> {
  let args: Vec<_> = std::env::args().collect();
  let extraction_mode = args.len() == 2;

  if args.len() != 3 && !extraction_mode {
    eprintln!("Invalid number of arguments");
    return Ok(());
  }

  if extraction_mode {
    let img = StegoImage::open(Path::new(&args[1]))?;

    let extracted = img.extract_data()?;

    std::io::stdout().write_all(&extracted)?;
  } else {
    let mut img = StegoImage::open(Path::new(&args[1]))?;

    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data)?;

    img.insert_data(&data)?;

    img.save(Path::new(&args[2]))?;
  }
  Ok(())
}
