# framework-region

提供国家、地区、电话前缀和联合国 M49 区域数据的静态访问能力。

## 安装

```toml
[dependencies]
framework-region = { path = "../framework-region" }
```

## 静态数据

- `regions::REGION_REGIONS`：国家和地区列表。
- `phones::REGION_PHONES`：号码前缀列表。
- `m49::REGION_M49`：联合国 M49 区域树根节点。

所有数据结构均实现了只复制静态引用的 `Clone`。

## 电话前缀匹配

`phones::match_phone` 接收字符串号码，会剔除开头的 `+`；
`phones::match_phone_number` 接收 `u64` 号码。两者均返回最长前缀对应的 `RegionPhone`。

```rust
use framework_region::phones::{match_phone, match_phone_number};

let china = match_phone("+8613800138000");
let china = match_phone_number(8613800138000);
```

## 数据来源

数据源来源于：[lingting/lingting-geo-data](https://github.com/lingting/lingting-geo-data)。
