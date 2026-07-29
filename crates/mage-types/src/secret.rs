use rand::Rng;

pub fn new() -> String {
    let mut bytes = vec![0u8; 8];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
