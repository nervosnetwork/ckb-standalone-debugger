pub fn println(s: &str) {
    web_sys::console::log_1(&s.into());
}
