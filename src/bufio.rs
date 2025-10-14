/// A buffered reader that can only ingest a limited amount of bytes from the provided handle
pub struct LimitedBufReader {
    // the buffer where reads are being stored
    buffer: Vec<u8>,
    /// the handle that BufReader is reading from
    handle: Box<dyn std::io::Read>,
    /// total number of bytes that BufReader has already read
    bytes_read: usize,
    /// the total number of bytes that BufReader can read
    limit: Option<usize>,
}

impl LimitedBufReader {
    pub fn new(buf_size: usize, handle: Box<dyn std::io::Read>, limit: Option<usize>) -> Self {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.resize(buf_size, 0);
        LimitedBufReader {
            buffer,
            handle,
            bytes_read: 0,
            limit,
        }
    }

    pub fn read(&mut self) -> Result<usize, std::io::Error> {
        if let Some(limit) = self.limit {
            if self.bytes_read >= limit {
                return Ok(0);
            }
        }

        let bytes_read = self.handle.read(&mut self.buffer).unwrap_or(0);

        if let Some(limit) = self.limit {
            let bytes_remaining = limit - self.bytes_read;
            if bytes_read > bytes_remaining {
                self.bytes_read += bytes_remaining;
                return Ok(bytes_remaining);
            }
        }

        self.bytes_read += bytes_read;
        Ok(bytes_read)
    }

    pub fn as_ref(&self) -> &Vec<u8> {
        return self.buffer.as_ref();
    }

    pub fn total_bytes_read(&self) -> usize {
        self.bytes_read
    }
}
