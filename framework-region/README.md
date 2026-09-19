# framework-region

提供国家、地区、电话前缀和联合国 M49 区域数据的静态访问能力。

## 安装

```toml
[dependencies]
framework-region = { path = "../framework-region" }
```

## 静态数据

| 常量 | 类型 | 内容 |
|------|------|------|
| `regions::REGION_REGIONS` | `RegionList` | 国家和地区列表 |
| `phones::REGION_PHONES` | `RegionPhoneList` | 号码前缀列表 |
| `m49::REGION_M49` | `RegionM49` | 联合国 M49 区域树根节点 |

`RegionList` 与 `RegionPhoneList` 提供 `as_slice()`（返回 `&'static [T]`）与 `iter()`（返回 `std::slice::Iter<'static, T>`）。

所有数据结构均实现了只复制静态引用的 `Clone`。

### 数据结构

| 类型 | 字段 |
|------|------|
| `RegionName` | `en`、`zh` |
| `RegionM49Code` | `region`、`subregion` |
| `Region` | `iso`、`iso3`、`flag`、`calling_codes`、`phone_prefixes`、`name`、`numeric`、`m49` |
| `RegionPhone` | `prefix`（`u64`）、`calling`（`u32`）、`region` |
| `RegionM49` | `code`、`name`、`children`、`regions` |

`RegionM49.children` 为下级区域节点，`regions` 为该节点直接包含的国家或地区 `iso` 列表。

```rust
use framework_region::m49::REGION_M49;
use framework_region::regions::REGION_REGIONS;

for region in REGION_REGIONS.iter() {
    let _ = (&region.iso, &region.name.zh);
}

for child in REGION_M49.children {
    let _ = (&child.code, child.regions.len());
}
```

## 电话前缀匹配

`phones::match_phone` 接收字符串号码，会剔除开头的 `+`，号码包含非 ASCII 数字字符时返回 `None`；
`phones::match_phone_number` 接收 `u64` 号码。两者均按最长电话前缀匹配，返回
`Option<&'static RegionPhone>`，未匹配时返回 `None`。

```rust
use framework_region::phones::{match_phone, match_phone_number};

let china = match_phone("+8613800138000");
let china = match_phone_number(8613800138000);
assert_eq!(china.map(|phone| phone.region), Some("CN"));
```

匹配使用惰性构建的前缀树：首次调用 `phones::phone_prefix_tree()` 时基于 `REGION_PHONES` 构建并缓存，
之后复用同一实例；也可直接调用该函数获取 `&'static PhonePrefixTree`。

## 数据生成

数据在构建期由 `build.rs` 从仓库根目录的 `assets/area/` 读取 JSON，生成 Rust 源码到 `OUT_DIR` 后由各模块
`include!`：

| 源文件 | 生成目标 |
|--------|----------|
| `assets/area/regions.json` | `regions_data.rs`，产出 `REGION_REGIONS` |
| `assets/area/phones.json` | `phones_data.rs`，产出 `REGION_PHONES` |
| `assets/area/m49.json` | `m49_data.rs`，产出 `REGION_M49` |

源文件变更会触发重新构建。

## 数据来源

数据源来源于：[lingting/lingting-geo-data](https://github.com/lingting/lingting-geo-data)。
