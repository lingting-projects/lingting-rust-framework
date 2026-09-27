```shell
# 同步新版本号
cargo workspaces version custom 26.8.15
# 发布
cargo release --workspace --registry crates-io
```