/// 把下划线或连字符分隔的名称转换为小驼峰名称。
pub fn camel_case(name: &str) -> String {
    let mut result = String::new();
    let mut uppercase_next = false;
    for (index, character) in name.chars().enumerate() {
        if character == '_' || character == '-' {
            uppercase_next = !result.is_empty();
            continue;
        }
        if index == 0 {
            result.extend(character.to_lowercase());
        } else if uppercase_next {
            result.extend(character.to_uppercase());
            uppercase_next = false;
        } else {
            result.push(character);
        }
    }
    result
}
