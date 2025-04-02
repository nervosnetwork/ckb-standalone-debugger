pub fn debug_printer(s: &str) {
    web_sys::console::log_1(&format!("Script log: {}", s).into());
}
