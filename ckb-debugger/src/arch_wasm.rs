pub fn println(s: &str) {
    web_sys::console::log_1(&s.into());
}

pub fn random() -> u64 {
    rand::random()
}

pub fn timestamp() -> u64 {
    web_sys::js_sys::Date::new_0().get_utc_seconds() as u64
}
