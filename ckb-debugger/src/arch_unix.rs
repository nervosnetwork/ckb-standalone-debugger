pub fn println(s: &str) {
    println!("{}", s);
}

pub fn random() -> u64 {
    rand::random()
}

pub fn timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs()
}
