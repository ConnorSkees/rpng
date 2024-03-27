use std::fs::File;
use std::io::{BufWriter, Cursor, Read, Write};
use std::path::Path;

use crc32fast::Hasher;
use flate2::bufread::ZlibEncoder;
use flate2::Compression;

use crate::chunks::Chunk;
use crate::common::{HEADER, IEND};
use crate::errors::PngDecodingError;
use crate::png::Png;

impl Png {
    pub fn save<S: AsRef<Path>>(&self, file_path: S) -> Result<(), PngDecodingError> {
        let buffer = &mut BufWriter::new(File::create(file_path)?);
        self.write(buffer)?;
        Ok(())
    }

    pub fn write<T: Write>(&self, buffer: &mut BufWriter<T>) -> Result<(), PngDecodingError> {
        buffer.write_all(&HEADER)?;
        self.write_chunk(&self.ihdr, buffer)?;
        self.write_data(buffer)?;
        buffer.write_all(&IEND)?;
        Ok(())
    }

    fn write_chunk<'a, T: Write>(
        &self,
        chunk: &impl Chunk<'a>,
        buffer: &mut BufWriter<T>,
    ) -> Result<(), PngDecodingError> {
        let serialized = chunk.serialize();
        let len = serialized.len() as u32 - 4;

        buffer.write_all(&len.to_be_bytes())?;
        buffer.write_all(&serialized)?;

        let mut hasher = Hasher::new();
        hasher.update(&serialized);
        buffer.write_all(&hasher.finalize().to_be_bytes())?;

        Ok(())
    }

    fn write_data<T: Write>(&self, buffer: &mut BufWriter<T>) -> Result<(), PngDecodingError> {
        let chunk = DataChunk {
            width: self.width(),
            height: self.height(),
            bpp: self.bpp(),
            raw_buffer: self
                .decoded_buffer
                .clone()
                .unwrap_or_else(|| self.decode().buffer),
        };

        self.write_chunk(&chunk, buffer)?;

        Ok(())
    }
}

struct DataChunk {
    raw_buffer: Vec<u8>,
    width: u32,
    height: u32,
    bpp: usize,
}

impl<'a> Chunk<'a> for DataChunk {
    const NAME: [u8; 4] = *b"IDAT";

    fn parse<T: Read + std::io::prelude::BufRead>(
        _length: u32,
        _buf: &mut T,
    ) -> Result<Self, PngDecodingError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> =
            Vec::with_capacity(4 + self.raw_buffer.len() + self.height as usize + 4);

        buffer.extend(b"IDAT");

        let chunks = self.raw_buffer.chunks_exact(self.width as usize * self.bpp);
        debug_assert_eq!(chunks.remainder(), &[]);

        let mut out = Vec::with_capacity(self.raw_buffer.len() + self.height as usize);

        for c in chunks {
            out.push(0);
            out.extend(c);
        }

        let mut compressor = ZlibEncoder::new(Cursor::new(out), Compression::fast());
        compressor.read_to_end(&mut buffer).unwrap();

        buffer
    }
}
