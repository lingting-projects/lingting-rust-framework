```shell
# 强制同步新版本号, 仅添加主tag
cargo workspaces version custom 26.8.141 --no-individual-tags --force '*' -y
# 发布
cargo release --workspace --registry crates-io
```