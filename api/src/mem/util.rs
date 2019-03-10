use ring::rand::{SecureRandom, SystemRandom};

// This string will be about twice as long as
pub fn secure_rand_hex(byte_len: usize) -> String {
    let mut nonce = vec![0; byte_len];
    SystemRandom::new().fill(&mut nonce).expect("Sucessful system random");
    hex(&nonce)
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    for &byte in bytes {
        write!(&mut s, "{:x}", byte).expect("Unable to write");
    }
    s
}
