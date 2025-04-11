pub fn file_read(name: &str) -> std::io::Result<Vec<u8>> {
    let s = web_sys::window().unwrap().local_storage().unwrap().unwrap().get(name).unwrap().unwrap();
    Ok(hex::decode(&s).unwrap())
}

pub fn file_write(name: &str, data: &[u8]) -> std::io::Result<()> {
    web_sys::window().unwrap().local_storage().unwrap().unwrap().set(name, &hex::encode(data)).unwrap();
    Ok(())
}

pub fn println(s: &str) {
    web_sys::console::log_1(&s.into());
}

pub fn random() -> u64 {
    rand::random()
}

pub fn timestamp() -> u64 {
    web_sys::js_sys::Date::new_0().get_utc_milliseconds() as u64 * 1000000
}
