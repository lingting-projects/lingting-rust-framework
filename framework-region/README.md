# framework-region

提供国家、地区、电话前缀和联合国 M49 区域数据的静态访问与查询能力。

## 安装

```toml
[dependencies]
framework-region = { path = "../framework-region" }
```

## 静态数据

- `REGION_REGIONS`：国家和地区列表。
- `REGION_PHONES`：号码前缀列表。
- `REGION_M49`：联合国 M49 区域树根节点。

所有数据结构均实现了只复制静态引用的 `Clone`。

## 查询

```rust
use framework_region::{find_by_calling_code, find_by_iso, find_by_phone_prefix};

let china = find_by_iso("CN");
let regions = find_by_calling_code("+86");
let phone = find_by_phone_prefix(8613800138000);
```

还提供 `find_by_iso3` 与 `find_by_m49`。

## 数据来源

数据源来源于：[lingting/lingting-geo-data](https://github.com/lingting/lingting-geo-data)。
