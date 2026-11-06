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

/// 把驼峰、下划线、连字符或空格分隔的名称转换为下划线小写名称。
pub fn snake_case(name: &str) -> String {
    let characters = name.chars().collect::<Vec<_>>();
    let mut result = String::new();
    for (index, character) in characters.iter().enumerate() {
        if *character == '_' || *character == '-' || *character == ' ' {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            continue;
        }
        if character.is_uppercase() {
            let previous = index.checked_sub(1).map(|index| characters[index]);
            let next = characters.get(index + 1);
            let separated = previous.is_some_and(|previous| {
                previous.is_lowercase()
                    || previous.is_ascii_digit()
                    || (previous.is_uppercase() && next.is_some_and(|next| next.is_lowercase()))
            });
            if separated && !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            result.extend(character.to_lowercase());
            continue;
        }
        result.push(*character);
    }
    result
}

/// 把名称转换为大写下划线名称，用于常量与静态项命名。
pub fn shouty_snake_case(name: &str) -> String {
    snake_case(name).to_uppercase()
}

/// 把下划线、连字符或空格分隔的名称转换为大驼峰名称。
pub fn pascal_case(name: &str) -> String {
    let mut result = String::new();
    let mut uppercase_next = true;
    for character in name.chars() {
        if character == '_' || character == '-' || character == ' ' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            result.extend(character.to_uppercase());
            uppercase_next = false;
        } else {
            result.push(character);
        }
    }
    result
}
