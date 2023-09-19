pub trait InputStream {
    type Lock: InputStreamLock;
    
    fn lock(&mut self) -> Self::Lock;
}

pub trait InputStreamLock {
    fn byte(&mut self) -> Option<u8>;
    fn bytes_buf(&mut self, buf: &mut [u8]) -> Option<()>;
    
    fn size(&mut self) -> Option<usize> {
        let mut size: usize = 0;

        loop {
            let byte = self.byte()?;

            size = size.checked_mul(128)?.checked_add((byte % 128) as usize)?;

            if byte < 128 {
                break;
            }
        }

        Some(size)
    }
    
    fn string(&mut self) -> Option<Vec<u8>> {
        let mut string: Vec<u8> = vec![0; self.size()?];

        self.bytes_buf(&mut string)?;

        Some(string)
    }
}

pub trait OutputStream {
    type Lock: OutputStreamLock;
    
    fn lock(&mut self) -> Self::Lock;
}

pub trait OutputStreamLock {
    fn output_byte(&mut self, byte: u8) -> Option<()>;
    fn output_bytes(&mut self, bytes: &[u8]) -> Option<()>;
    
    fn output_size(&mut self, mut size: usize) -> Option<()> {
        let bytes: Vec<u8> = Vec::with_capacity((usize::BITS >> 3) as usize);
        
        bytes.push((0x80 | size % 128) as u8);
        
        while size != 0 {
            bytes.push((size % 128) as u8);
            
            size >>= 7;
        }
        
        bytes.reverse();
        
        self.output_bytes(&bytes)?;
        
        Some(())
    }
    
    fn output_string(&mut self, string: &[u8]) -> Option<()> {
        self.output_size(string.len())?;
        self.output_bytes(string)?;
        
        Some(())
    }
}