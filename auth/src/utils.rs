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

use short_crypt::ShortCrypt;

pub fn enc(src: &[u8], key: &str) -> String {
    // I'm sorry
    ShortCrypt::new(key).encrypt_to_url_component(src)
}

pub fn dec(enc: &str, key: &str) -> Result<Vec<u8>, &'static str> {
    ShortCrypt::new(key).decrypt_url_component(enc)
}
