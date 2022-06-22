use std::{
  collections::hash_map::DefaultHasher,
  error::Error,
  fmt::Display,
  hash::{Hash, Hasher},
  path::Path,
};

use image::{io::Reader, ImageResult, Rgba};

type ImageBuffer = image::ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug)]
pub enum StegoError {
  NotEnoughSpace,
  NothingToInsert,
  InvalidDataLength,
  TooSmallImage,
  InvalidHashCheck,
}

impl Display for StegoError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(match self {
      StegoError::NotEnoughSpace => "Not enough space in image to insert requested data",
      StegoError::NothingToInsert => "Supplied data contains zero bytes",
      StegoError::InvalidDataLength => "Image contains invalid data length marker",
      StegoError::TooSmallImage => "Image is too small to contain any data",
      StegoError::InvalidHashCheck => "Image is too small to contain any data",
    })
  }
}

impl Error for StegoError {}

pub type StegoResult<T> = Result<T, StegoError>;

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

  const HEADER_SIZE: usize = std::mem::size_of::<usize>() + std::mem::size_of::<u64>();
  pub fn avaliable(&self) -> usize {
    (self.img.width() as usize * self.img.height() as usize)
      .checked_sub(Self::HEADER_SIZE)
      .unwrap_or_default()
  }

  fn calculate_hash(data: &[u8]) -> StegoResult<u64> {
    let mut hasher = DefaultHasher::new();
    for byte in data {
      byte.hash(&mut hasher);
    }
    Ok(hasher.finish())
  }

  pub fn insert_data(&mut self, data: &[u8]) -> StegoResult<()> {
    if data.len() == 0 {
      return Err(StegoError::NothingToInsert);
    }
    if self.avaliable() < data.len() {
      return Err(StegoError::NotEnoughSpace);
    }
    let data_len_bytes = data.len().to_le_bytes();
    let hash_bytes = Self::calculate_hash(data)?.to_le_bytes();
    let data = data_len_bytes.iter().chain(&hash_bytes).chain(data);
    for (pixel, data) in self.img.chunks_exact_mut(4).zip(data) {
      for (i, channel) in pixel.iter_mut().enumerate() {
        *channel &= !0x3;
        *channel |= *data >> (i << 1) & 0x3;
      }
    }
    Ok(())
  }

  fn extract_size(&self) -> StegoResult<usize> {
    let channels = self.img.chunks_exact(4);
    let mut extracted_size_bytes = [0u8; std::mem::size_of::<usize>()];
    for (i, pixel) in channels.take(std::mem::size_of::<usize>()).enumerate() {
      for (j, channel) in pixel.iter().enumerate() {
        extracted_size_bytes[i] |= (channel & 0x3) << (j << 1);
      }
    }
    let extracted_size = usize::from_le_bytes(extracted_size_bytes);
    if extracted_size > self.avaliable() {
      return Err(StegoError::InvalidDataLength);
    }
    Ok(extracted_size)
  }

  fn extract_hash(&self) -> StegoResult<u64> {
    let channels = self.img.chunks_exact(4);
    let mut extracted_hash_bytes = [0u8; std::mem::size_of::<u64>()];
    for (i, pixel) in channels
      .skip(std::mem::size_of::<usize>())
      .take(std::mem::size_of::<u64>())
      .enumerate()
    {
      for (j, channel) in pixel.iter().enumerate() {
        extracted_hash_bytes[i] |= (channel & 0x3) << (j << 1);
      }
    }
    Ok(u64::from_le_bytes(extracted_hash_bytes))
  }

  pub fn extract_data(&self) -> StegoResult<Vec<u8>> {
    let channels = self.img.chunks_exact(4);
    if channels.len() < Self::HEADER_SIZE {
      return Err(StegoError::TooSmallImage);
    }
    let extracted_size = self.extract_size()?;
    let extracted_hash = self.extract_hash()?;

    let mut data = vec![0u8; extracted_size];
    for (i, pixel) in channels
      .skip(Self::HEADER_SIZE)
      .take(extracted_size)
      .enumerate()
    {
      for (j, channel) in pixel.iter().enumerate() {
        data[i] |= (channel & 0x3) << (j << 1);
      }
    }
    if Self::calculate_hash(&data)? != extracted_hash {
      return Err(StegoError::InvalidHashCheck);
    }
    Ok(data)
  }
}
