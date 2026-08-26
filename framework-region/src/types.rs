//! 地区静态数据类型。

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
