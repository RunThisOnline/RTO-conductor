/*use crate::Result;

pub fn byte(src: impl Read) -> Result<u8> {
    let mut buf: [u8; 1] = [0];

    stdin.read_exact(&mut buf)?;

    buf[0]
}

pub fn int(src: impl Read) -> Result<u8> {
    let mut int: usize = 0;

    loop {
        let byte = byte();

        int = int.checked_mul(128)?.checked_add((byte % 128) as usize)?;

        if byte < 128 {
            break;
        }
    }

    int
}

pub fn string(src: impl Read) -> Result<u8> {
    
}

pub fn write_byte(to: impl Write, byte: u8) -> Result<()> {
    stdin.write_all()?;
}*/