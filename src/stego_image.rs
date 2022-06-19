use std::{error::Error, fmt::Display, path::Path};

use image::{io::Reader, ImageResult, Rgba};

type ImageBuffer = image::ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug)]
pub enum StegoError {
  NotEnoughSpace,
  NothingToInsert,
  InvalidDataLength,
  TooSmallImage,
}

impl Display for StegoError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      StegoError::NotEnoughSpace => {
        f.write_str("Not enough space in image to insert requested data")
      }
      StegoError::NothingToInsert => f.write_str("Supplied data contains zero bytes"),
      StegoError::InvalidDataLength => f.write_str("Image contains invalid data length marker"),
      StegoError::TooSmallImage => f.write_str("Image is too small to contain any data"),
    }
  }
}

impl Error for StegoError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }

  fn description(&self) -> &str {
    "description() is deprecated; use Display"
  }

  fn cause(&self) -> Option<&dyn Error> {
    self.source()
  }
}

pub struct StegoImage {
  img: ImageBuffer,
}

impl StegoImage {
  pub fn open(path: &Path) -> ImageResult<Self> {
    let img = Reader::open(path)?.decode()?.to_rgba8();
    Ok(Self { img })
  }

  pub fn save(&self, path: &Path) -> ImageResult<()> {
    self.img.save(path)
  }

  pub fn avaliable(&self) -> usize {
    (self.img.width() as usize * self.img.height() as usize)
      .checked_sub(std::mem::size_of::<usize>())
      .unwrap_or_default()
  }

  pub fn insert_data(&mut self, data: &[u8]) -> Result<(), StegoError> {
    if data.len() == 0 {
      return Err(StegoError::NothingToInsert);
    }
    if self.avaliable() < data.len() {
      return Err(StegoError::NotEnoughSpace);
    }
    let data_len_bytes = data.len().to_le_bytes();
    let data = data_len_bytes.iter().chain(data);
    for (pixel, data) in self.img.chunks_exact_mut(4).zip(data) {
      for (i, channel) in pixel.iter_mut().enumerate() {
        *channel &= !0x3;
        *channel |= *data >> (i << 1) & 0x3;
      }
    }
    Ok(())
  }

  pub fn extract_data(&self) -> Result<Vec<u8>, StegoError> {
    let channels = self.img.chunks_exact(4);
    if channels.len() < std::mem::size_of::<usize>() {
      return Err(StegoError::TooSmallImage);
    }
    let mut extracted_size_bytes = [0u8; std::mem::size_of::<usize>()];
    for (i, pixel) in channels
      .clone()
      .take(std::mem::size_of::<usize>())
      .enumerate()
    {
      for (j, channel) in pixel.iter().enumerate() {
        extracted_size_bytes[i] |= (channel & 0x3) << (j << 1);
      }
    }
    let extracted_size = usize::from_le_bytes(extracted_size_bytes);
    if extracted_size > self.avaliable() {
      return Err(StegoError::InvalidDataLength);
    }
    let mut data = vec![0u8; extracted_size];
    for (i, pixel) in channels
      .skip(std::mem::size_of::<usize>())
      .take(extracted_size)
      .enumerate()
    {
      for (j, channel) in pixel.iter().enumerate() {
        data[i] |= (channel & 0x3) << (j << 1);
      }
    }
    Ok(data)
  }
}
