```shell
# 强制同步新版本号, 仅添加主tag
bash scripts/tag.sh  26.8.142
# 发布 - cargo workspace 在 ci 中使用要切换到分支, tag触发的没法直接用
bash scripts/publish.sh
```