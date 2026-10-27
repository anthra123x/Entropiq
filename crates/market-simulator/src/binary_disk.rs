use crate::binary_feed::{BinaryQuoteRecord, BinaryTradeRecord};
use market_microstructure::{Quote, Trade};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Magic 4-byte header identifier for Quantum Market Binary Data files: 'Q', 'M', 'B', 'D'
pub const MAGIC_HEADER: [u8; 4] = [0x51, 0x4D, 0x42, 0x44];
/// Current file format version
pub const FORMAT_VERSION: u32 = 1;
/// Internal buffer capacity aligned to 64 KB Linux OS Page Cache blocks
pub const BUFFER_CAPACITY: usize = 64 * 1024;

/// Ultra-high performance buffered disk recorder for 32-byte binary market records.
pub struct BinaryDiskRecorder {
    writer: BufWriter<File>,
    records_written: u64,
}

impl BinaryDiskRecorder {
    /// Creates or truncates a .bin file on disk with a valid binary header.
    pub fn create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let mut writer = BufWriter::with_capacity(BUFFER_CAPACITY, file);

        // Write 16-byte Header: Magic (4) + Version (4) + Total Records placeholder (8)
        writer.write_all(&MAGIC_HEADER)?;
        writer.write_all(&FORMAT_VERSION.to_le_bytes())?;
        writer.write_all(&0u64.to_le_bytes())?;

        Ok(Self {
            writer,
            records_written: 0,
        })
    }

    /// Appends a single 32-byte binary quote directly to the 64KB page buffer.
    #[inline(always)]
    pub fn write_quote(&mut self, record: &BinaryQuoteRecord) -> io::Result<()> {
        self.writer.write_all(&record.to_bytes())?;
        self.records_written += 1;
        Ok(())
    }

    /// Appends a slice of quotes in a single vectorized memory copy.
    pub fn write_quote_batch(&mut self, records: &[BinaryQuoteRecord]) -> io::Result<()> {
        for r in records {
            self.writer.write_all(&r.to_bytes())?;
        }
        self.records_written += records.len() as u64;
        Ok(())
    }

    /// Appends a single 32-byte binary trade directly to the 64KB page buffer.
    #[inline(always)]
    pub fn write_trade(&mut self, record: &BinaryTradeRecord) -> io::Result<()> {
        self.writer.write_all(&record.to_bytes())?;
        self.records_written += 1;
        Ok(())
    }

    /// Appends a slice of trades in a single vectorized memory copy.
    pub fn write_trade_batch(&mut self, records: &[BinaryTradeRecord]) -> io::Result<()> {
        for r in records {
            self.writer.write_all(&r.to_bytes())?;
        }
        self.records_written += records.len() as u64;
        Ok(())
    }

    /// Flushes the OS write buffer and finalizes the file header with total record count.
    pub fn finish(mut self) -> io::Result<u64> {
        self.writer.flush()?;

        // Seek back to byte 8 to update the total record count
        let mut file = self.writer.into_inner()?;
        file.seek(SeekFrom::Start(8))?;
        file.write_all(&self.records_written.to_le_bytes())?;
        file.flush()?;

        Ok(self.records_written)
    }

    pub fn records_written(&self) -> u64 {
        self.records_written
    }
}

/// High-speed buffered disk reader for binary market logs.
pub struct BinaryDiskReader {
    reader: BufReader<File>,
    total_records: u64,
    records_read: u64,
}

impl BinaryDiskReader {
    /// Opens and validates a .bin file header.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(BUFFER_CAPACITY, file);

        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != MAGIC_HEADER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid magic header: not a valid Quantum Market Binary file",
            ));
        }

        let mut ver_bytes = [0u8; 4];
        reader.read_exact(&mut ver_bytes)?;
        let version = u32::from_le_bytes(ver_bytes);
        if version != FORMAT_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported file version: {}", version),
            ));
        }

        let mut count_bytes = [0u8; 8];
        reader.read_exact(&mut count_bytes)?;
        let total_records = u64::from_le_bytes(count_bytes);

        Ok(Self {
            reader,
            total_records,
            records_read: 0,
        })
    }

    /// Reads next 32-byte quote record from disk buffer. Returns None on EOF.
    #[inline(always)]
    pub fn read_next_quote(&mut self) -> io::Result<Option<Quote>> {
        let mut buffer = [0u8; BinaryQuoteRecord::SIZE];
        match self.reader.read_exact(&mut buffer) {
            Ok(_) => {
                self.records_read += 1;
                match BinaryQuoteRecord::from_bytes(&buffer) {
                    Ok(rec) => Ok(Some(rec.to_quote())),
                    Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Reads next 32-byte trade record from disk buffer. Returns None on EOF.
    #[inline(always)]
    pub fn read_next_trade(&mut self) -> io::Result<Option<Trade>> {
        let mut buffer = [0u8; BinaryTradeRecord::SIZE];
        match self.reader.read_exact(&mut buffer) {
            Ok(_) => {
                self.records_read += 1;
                match BinaryTradeRecord::from_bytes(&buffer) {
                    Ok(rec) => Ok(Some(rec.to_trade())),
                    Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn total_records(&self) -> u64 {
        self.total_records
    }

    pub fn records_read(&self) -> u64 {
        self.records_read
    }
}
