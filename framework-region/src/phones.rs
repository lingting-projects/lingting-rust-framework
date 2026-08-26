//! 电话前缀数据与最长前缀匹配。

use std::sync::OnceLock;

use crate::types::{RegionPhone, RegionPhoneList};

include!(concat!(env!("OUT_DIR"), "/phones_data.rs"));

static PHONE_PREFIX_TREE: OnceLock<PhonePrefixTree> = OnceLock::new();

/// 按号码字符串匹配电话归属地区。
///
/// 会剔除号码开头的 `+`，并按最长电话前缀返回匹配项。
pub fn match_phone(phone: &str) -> Option<&'static RegionPhone> {
    let phone = phone.strip_prefix('+').unwrap_or(phone);
    phone
        .bytes()
        .all(|digit| digit.is_ascii_digit())
        .then(|| phone_prefix_tree().match_phone(phone))
        .flatten()
}

/// 按数字号码匹配电话归属地区。
///
/// 按最长电话前缀返回匹配项。
pub fn match_phone_number(phone: u64) -> Option<&'static RegionPhone> {
    phone_prefix_tree().match_phone(&phone.to_string())
}

pub fn phone_prefix_tree() -> &'static PhonePrefixTree {
    PHONE_PREFIX_TREE.get_or_init(|| PhonePrefixTree::new(REGION_PHONE_VALUES))
}

#[derive(Default)]
pub struct PhonePrefixTree {
    roots: Vec<PhonePrefixNode>,
}

impl PhonePrefixTree {
    fn new(values: &'static [RegionPhone]) -> Self {
        let mut root = PhonePrefixBuildNode::default();
        for phone in values {
            root.insert(phone);
        }

        Self {
            roots: root.compress_children(),
        }
    }

    fn match_phone(&self, phone: &str) -> Option<&'static RegionPhone> {
        let mut nodes = &self.roots;
        let mut remaining = phone.as_bytes();
        let mut matched = None;

        while let Some(node) = nodes
            .iter()
            .find(|node| remaining.starts_with(node.digits.as_bytes()))
        {
            remaining = &remaining[node.digits.len()..];
            if node.phone.is_some() {
                matched = node.phone;
            }
            nodes = &node.children;
        }

        matched
    }
}

#[derive(Default)]
struct PhonePrefixBuildNode {
    phone: Option<&'static RegionPhone>,
    children: [Option<Box<PhonePrefixBuildNode>>; 10],
}

impl PhonePrefixBuildNode {
    fn insert(&mut self, phone: &'static RegionPhone) {
        let mut node = self;
        for digit in phone.prefix.to_string().bytes() {
            let index = usize::from(digit - b'0');
            node = node.children[index].get_or_insert_default();
        }
        node.phone.get_or_insert(phone);
    }

    fn compress_children(self) -> Vec<PhonePrefixNode> {
        self.children
            .into_iter()
            .enumerate()
            .filter_map(|(digit, child)| child.map(|child| child.compress(digit as u8 + b'0')))
            .collect()
    }

    fn compress(self, digit: u8) -> PhonePrefixNode {
        let mut digits = String::from(char::from(digit));
        let mut node = self;

        while node.phone.is_none() && node.children.iter().flatten().count() == 1 {
            let (digit, child) = node
                .children
                .into_iter()
                .enumerate()
                .find_map(|(digit, child)| child.map(|child| (digit as u8 + b'0', child)))
                .expect("子节点数量已确认");
            digits.push(char::from(digit));
            node = *child;
        }

        PhonePrefixNode {
            digits,
            phone: node.phone,
            children: node.compress_children(),
        }
    }
}

struct PhonePrefixNode {
    digits: String,
    phone: Option<&'static RegionPhone>,
    children: Vec<PhonePrefixNode>,
}
