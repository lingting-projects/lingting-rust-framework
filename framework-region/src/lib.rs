//! 国家、地区、电话前缀与联合国 M49 区域数据。

/// 多语言地区名称。
#[derive(Debug, PartialEq, Eq)]
pub struct RegionName {
    pub en: &'static str,
    pub zh: &'static str,
}

impl Clone for RegionName {
    fn clone(&self) -> Self {
        Self {
            en: self.en,
            zh: self.zh,
        }
    }
}

/// 地区对应的 M49 区域和子区域编码。
#[derive(Debug, PartialEq, Eq)]
pub struct RegionM49Code {
    pub region: &'static str,
    pub subregion: &'static str,
}

impl Clone for RegionM49Code {
    fn clone(&self) -> Self {
        Self {
            region: self.region,
            subregion: self.subregion,
        }
    }
}

/// 国家或地区数据。
#[derive(Debug, PartialEq, Eq)]
pub struct Region {
    pub iso: &'static str,
    pub iso3: &'static str,
    pub flag: &'static str,
    pub calling_codes: &'static [&'static str],
    pub phone_prefixes: &'static [&'static str],
    pub name: RegionName,
    pub numeric: &'static str,
    pub m49: RegionM49Code,
}

impl Clone for Region {
    fn clone(&self) -> Self {
        Self {
            iso: self.iso,
            iso3: self.iso3,
            flag: self.flag,
            calling_codes: self.calling_codes,
            phone_prefixes: self.phone_prefixes,
            name: self.name.clone(),
            numeric: self.numeric,
            m49: self.m49.clone(),
        }
    }
}

/// 国家或地区静态列表。
#[derive(Debug)]
pub struct RegionList {
    values: &'static [Region],
}

impl RegionList {
    pub const fn new(values: &'static [Region]) -> Self {
        Self { values }
    }

    pub const fn as_slice(&self) -> &'static [Region] {
        self.values
    }

    pub fn iter(&self) -> std::slice::Iter<'static, Region> {
        self.values.iter()
    }
}

impl Clone for RegionList {
    fn clone(&self) -> Self {
        Self {
            values: self.values,
        }
    }
}

/// 电话前缀记录。
#[derive(Debug, PartialEq, Eq)]
pub struct RegionPhone {
    pub prefix: u64,
    pub calling: u32,
    pub region: &'static str,
}

impl Clone for RegionPhone {
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix,
            calling: self.calling,
            region: self.region,
        }
    }
}

/// 电话前缀静态列表。
#[derive(Debug)]
pub struct RegionPhoneList {
    values: &'static [RegionPhone],
}

impl RegionPhoneList {
    pub const fn new(values: &'static [RegionPhone]) -> Self {
        Self { values }
    }

    pub const fn as_slice(&self) -> &'static [RegionPhone] {
        self.values
    }

    pub fn iter(&self) -> std::slice::Iter<'static, RegionPhone> {
        self.values.iter()
    }
}

impl Clone for RegionPhoneList {
    fn clone(&self) -> Self {
        Self {
            values: self.values,
        }
    }
}

/// 联合国 M49 区域树节点。
#[derive(Debug, PartialEq, Eq)]
pub struct RegionM49 {
    pub code: &'static str,
    pub name: RegionName,
    pub children: &'static [RegionM49],
    pub regions: &'static [&'static str],
}

impl Clone for RegionM49 {
    fn clone(&self) -> Self {
        Self {
            code: self.code,
            name: self.name.clone(),
            children: self.children,
            regions: self.regions,
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/region_data.rs"));

/// 按 ISO 3166-1 alpha-2 编码查询地区。
pub fn find_by_iso(iso: &str) -> Option<Region> {
    REGION_REGIONS
        .iter()
        .find(|region| region.iso.eq_ignore_ascii_case(iso))
        .cloned()
}

/// 按 ISO 3166-1 alpha-3 编码查询地区。
pub fn find_by_iso3(iso3: &str) -> Option<Region> {
    REGION_REGIONS
        .iter()
        .find(|region| region.iso3.eq_ignore_ascii_case(iso3))
        .cloned()
}

/// 按国家电话区号查询所有匹配地区。
pub fn find_by_calling_code(calling_code: &str) -> Vec<Region> {
    let calling_code = calling_code.trim_start_matches('+');
    REGION_REGIONS
        .iter()
        .filter(|region| region.calling_codes.contains(&calling_code))
        .cloned()
        .collect()
}

/// 按完整号码前缀查询电话归属地区。
pub fn find_by_phone_prefix(prefix: u64) -> Option<RegionPhone> {
    REGION_PHONES
        .iter()
        .find(|phone| phone.prefix == prefix)
        .cloned()
}

/// 按 M49 编码查询区域树节点。
pub fn find_by_m49(code: &str) -> Option<RegionM49> {
    find_m49_node(&REGION_M49, code)
}

fn find_m49_node(node: &'static RegionM49, code: &str) -> Option<RegionM49> {
    if node.code == code {
        return Some(node.clone());
    }

    node.children
        .iter()
        .find_map(|child| find_m49_node(child, code))
}
