pub fn to_hex(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);

    for byte in s.as_bytes() {
        result.push_str(&format!("{byte:02x}"));
    }

    result
}
