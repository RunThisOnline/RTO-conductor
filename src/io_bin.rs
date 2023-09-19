use std::io::{Read, Write};

pub trait LockedInputStream {
    type Lock: InputStream;
    
    fn lock(&mut self) -> Self::Lock;
}

pub trait InputStream {
    fn input_byte(&mut self) -> Result<u8, ()>;
    fn input_bytes_buf(&mut self, buf: &mut [u8]) -> Result<(), ()>;
    
    fn input_size(&mut self) -> Result<usize, ()> {
        let mut size: usize = 0;

        loop {
            let byte = self.input_byte()?;

            size = size.checked_mul(128).ok_or(())?.checked_add((byte % 128) as usize).ok_or(())?;

            if byte < 128 {
                break;
            }
        }

        Ok(size)
    }
    
    fn input_string(&mut self) -> Result<Vec<u8>, ()> {
        let mut string: Vec<u8> = vec![0; self.input_size()?];

        self.input_bytes_buf(&mut string)?;

        Ok(string)
    }
}

impl<T> InputStream for T where T: Read {
    fn input_byte(&mut self) -> Result<u8, ()> {
        let mut buf = [0u8];

        self.read_exact(&mut buf).map_err(|_| ())?;

        Ok(buf[0])
    }

    fn input_bytes_buf(&mut self, buf: &mut [u8]) -> Result<(), ()> {
        self.read_exact(&mut buf).map_err(|_| ())
    }
}

pub trait LockedOutputStream {
    type Lock: OutputStream;
    
    fn lock(&mut self) -> Self::Lock;
}

pub trait OutputStream {
    fn output_byte(&mut self, byte: u8) -> Result<(), ()>;
    fn output_bytes(&mut self, bytes: &[u8]) -> Result<(), ()>;
    
    fn output_size(&mut self, mut size: usize) -> Result<(), ()> {
        let bytes: Vec<u8> = Vec::with_capacity((usize::BITS >> 3) as usize);
        
        bytes.push((0x80 | size % 128) as u8);
        
        while size != 0 {
            bytes.push((size % 128) as u8);
            
            size >>= 7;
        }
        
        bytes.reverse();
        
        self.output_bytes(&bytes)?;
        
        Ok(())
    }
    
    fn output_string(&mut self, string: &[u8]) -> Result<(), ()> {
        self.output_size(string.len())?;
        self.output_bytes(string)?;
        
        Ok(())
    }
}

impl<T> OutputStream for T where T: Write {
    fn output_byte(&mut self, byte: u8) -> Result<(), ()> {
        self.write_all(&[byte]).map_err(|_| ())
    }

    fn output_bytes(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.write_all(bytes).map_err(|_| ())
    }
}