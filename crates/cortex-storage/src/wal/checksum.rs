pub type Crc32c = u32;

pub fn crc32c(bytes: &[u8]) -> Crc32c {
    crc32c::crc32c(bytes)
}

pub fn align8(len: usize) -> usize {
    (len + 7) & !7
}
