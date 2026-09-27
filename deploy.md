```shell
# 强制同步新版本号, 仅添加主tag
bash scripts/tag.sh  26.8.142
# 发布
cargo release --workspace --registry crates-io
```