use ring::rand::{SecureRandom, SystemRandom};

// This string will be about twice as long as byte_len
pub fn secure_rand_hex(byte_len: usize) -> String {
    hex(&secure_rand(byte_len))
}

pub fn secure_rand(byte_len: usize) -> Vec<u8> {
    let mut res = vec![0; byte_len];
    SystemRandom::new()
        .fill(&mut res)
        .expect("Sucessful system random");
    res
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    for &byte in bytes {
        write!(&mut s, "{:02x}", byte).expect("Unable to write");
    }
    s
}
