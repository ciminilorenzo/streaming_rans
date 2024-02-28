use dsi_bitstream::{
    impls::{BufBitReader, BufBitWriter, WordAdapter},
    traits::{BitRead, BitWrite, Endianness, WordRead, BE, LE},
};
use mmap_rs::*;

use std::{fs::File, io::BufWriter, path::Path};
use webgraph::utils::MmapBackend;

#[inline(always)]
/// Write a value using an encoding table.
///
/// If the result is `Some` the encoding was successful, and
/// length of the code is returned.
pub fn write_table_le<B: BitWrite<LE>>(
    backend: &mut B,
    value: u64,
) -> Result<Option<usize>, B::Error> {
    Ok(if let Some(bits) = WRITE_LE.get(value as usize) {
        let len = WRITE_LEN_LE[value as usize] as usize;
        backend.write_bits(*bits as u64, len)?;
        Some(len)
    } else {
        None
    })
}

#[inline(always)]
/// Write a value using an encoding table.
///
/// If the result is `Some` the encoding was successful, and
/// length of the code is returned.
pub fn write_table_be<B: BitWrite<BE>>(
    backend: &mut B,
    value: u64,
) -> Result<Option<usize>, B::Error> {
    Ok(if let Some(bits) = WRITE_BE.get(value as usize) {
        let len = WRITE_LEN_BE[value as usize] as usize;
        backend.write_bits(*bits as u64, len)?;
        Some(len)
    } else {
        None
    })
}

///Table used to speed up the writing of gamma codes
pub const WRITE_BE: &[u16] = &[
    1, 2, 6, 4, 12, 20, 28, 8, 24, 40, 56, 72, 88, 104, 120, 16, 48, 80, 112, 144, 176, 208, 240,
    272, 304, 336, 368, 400, 432, 464, 496, 32, 96, 160, 224, 288, 352, 416, 480, 544, 608, 672,
    736, 800, 864, 928, 992, 1056, 1120, 1184, 1248, 1312, 1376, 1440, 1504, 1568, 1632, 1696,
    1760, 1824, 1888, 1952, 2016, 64,
];
///Table used to speed up the writing of gamma codes
pub const WRITE_LEN_BE: &[u16] = &[
    1, 3, 3, 5, 5, 5, 5, 7, 7, 7, 7, 7, 7, 7, 7, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 13,
];
///Table used to speed up the writing of gamma codes
pub const WRITE_LE: &[u16] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64,
];
///Table used to speed up the writing of gamma codes
pub const WRITE_LEN_LE: &[u16] = &[
    1, 3, 3, 5, 5, 5, 5, 7, 7, 7, 7, 7, 7, 7, 7, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 13,
];

/// Trait for writing reverse γ codes.
pub trait GammaRevWrite<E: Endianness>: BitWrite<E> {
    fn write_rev_gamma(&mut self, n: u64) -> Result<usize, Self::Error>;
}

impl<B: BitWrite<BE>> GammaRevWrite<BE> for B {
    #[inline]
    #[allow(clippy::collapsible_if)]
    fn write_rev_gamma(&mut self, n: u64) -> Result<usize, Self::Error> {
        if let Some(len) = write_table_be(self, n)? {
            return Ok(len);
        }
        default_rev_write_gamma(self, n)
    }
}

impl<B: BitWrite<LE>> GammaRevWrite<LE> for B {
    #[inline]
    #[allow(clippy::collapsible_if)]
    fn write_rev_gamma(&mut self, n: u64) -> Result<usize, Self::Error> {
        if let Some(len) = write_table_le(self, n)? {
            eprint!("***");
            return Ok(len);
        }
        default_rev_write_gamma(self, n)
    }
}

#[inline(always)]
fn default_rev_write_gamma<E: Endianness, B: BitWrite<E>>(
    backend: &mut B,
    mut n: u64,
) -> Result<usize, B::Error> {
    n += 1;
    let number_of_bits_to_write = n.ilog2();

    Ok(backend.write_bits(n, number_of_bits_to_write as _)?
        + backend.write_bits(1, 1)?
        + backend.write_bits(0, number_of_bits_to_write as _)?)
}

pub struct RevBitWriter<P: AsRef<Path>> {
    path: P,
    bit_writer: BufBitWriter<BE, WordAdapter<u64, BufWriter<File>>>,
}

impl<P: AsRef<Path>> RevBitWriter<P> {
    pub fn new(path: P) -> anyhow::Result<Self> {
        let bit_writer = BufBitWriter::new(WordAdapter::new(BufWriter::new(File::create(
            path.as_ref(),
        )?)));

        Ok(Self { path, bit_writer })
    }

    pub fn push(&mut self, x: u64) -> anyhow::Result<usize> {
        Ok(self.bit_writer.write_rev_gamma(x)?)
    }

    pub fn flush(mut self) -> anyhow::Result<BufBitReader<LE, RevReader>> {
        let padding = u64::BITS as usize - self.bit_writer.flush()?;
        let mut rev_reader = BufBitReader::<LE, _, _>::new(RevReader::new(self.path)?);
        rev_reader.skip_bits(padding as usize)?;
        Ok(rev_reader)
    }
}

pub struct RevReader {
    mmap: MmapBackend<u32>,
    position: usize,
}

impl RevReader {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mmap = MmapBackend::<u32>::load(path, MmapFlags::SEQUENTIAL)?;
        let position = mmap.as_ref().len();
        Ok(Self { mmap, position })
    }
}

impl WordRead for RevReader {
    type Word = u32;
    type Error = std::io::Error;
    fn read_word(&mut self) -> std::io::Result<u32> {
        if self.position == 0 {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No more data to read",
            ))
        } else {
            self.position -= 1;
            let w = self.mmap.as_ref()[self.position].to_be();
            Ok(w)
        }
    }
}

#[test]
fn test_rev() -> anyhow::Result<()> {
    use dsi_bitstream::codes::GammaRead;
    use rand::rngs::SmallRng;
    use rand::RngCore;
    use rand::SeedableRng;
    let tmp = tempfile::NamedTempFile::new()?;
    let mut rev_writer = RevBitWriter::new(tmp)?;

    let mut v = vec![];

    let mut r = SmallRng::seed_from_u64(42);

    for _ in 0..1000 {
        let x = r.next_u64() % 1024;
        v.push(x);
        rev_writer.push(x);
    }

    let mut rev_reader = rev_writer.flush()?;

    for &x in v.iter().rev() {
        let y = rev_reader.read_gamma()?;
        assert_eq!(y, x);
    }

    Ok(())
}
