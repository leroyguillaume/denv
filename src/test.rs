// IMPORTS

use std::io::{self, Write};

// STRUCTS

pub struct WriteFailer;

impl Write for WriteFailer {
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}
