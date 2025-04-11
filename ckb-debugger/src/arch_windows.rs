use std::io::{Read, Write};

pub fn file_read(name: &str) -> std::io::Result<Vec<u8>> {
    if name == "-" {
        let mut v = Vec::<u8>::new();
        let mut stdin = std::io::stdin();
        stdin.read_to_end(&mut v)?;
        Ok(v)
    } else {
        std::fs::read(name)
    }
}

pub fn file_write(name: &str, data: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::File::create(name)?;
    file.write_all(&data)?;
    Ok(())
}

pub fn println(s: &str) {
    println!("{}", s);
}

pub fn random() -> u64 {
    rand::random()
}

pub fn timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64
}
