#!/usr/bin/env bash
#
# 更新根 Cargo.toml 中的版本号、同步 workspace crate 的 Cargo.lock，
# 并可选择性地提交、打 tag、推送到远程。
#
# 用法:
#   bash scripts/tag.sh <version>       # 全流程: 改版本 -> 更新 lock -> commit -> tag -> push
#   bash scripts/tag.sh -d <version>    # 只改版本 + 更新 lock, 不执行 git 相关操作
#
# 示例:
#   bash scripts/tag.sh 26.8.142
#   bash scripts/tag.sh -d 26.8.142

set -euo pipefail

# ---------- 参数解析 ----------
DRY_RUN=false
POSITIONAL=()
for arg in "$@"; do
  case "$arg" in
    -d|--dry-run) DRY_RUN=true ;;
    *)            POSITIONAL+=("$arg") ;;
  esac
done

VERSION="${POSITIONAL[0]:-}"
if [[ -z "$VERSION" ]]; then
  echo "用法: bash scripts/tag.sh [-d] <version>" >&2
  echo "示例: bash scripts/tag.sh 26.8.142" >&2
  exit 1
fi

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+].*)?$ ]]; then
  echo "错误: 版本号格式不合法: $VERSION (期望形如 26.8.142)" >&2
  exit 1
fi

# ---------- 定位路径 ----------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$ROOT_DIR/Cargo.toml"

if [[ ! -f "$CARGO_TOML" ]]; then
  echo "错误: 未找到 $CARGO_TOML" >&2
  exit 1
fi

echo "==> 目标版本: $VERSION"
echo "==> Cargo.toml: $CARGO_TOML"

# ---------- 步骤 1 & 2: 修改根 Cargo.toml ----------
TMP_FILE="$(mktemp)"
trap 'rm -f "$TMP_FILE"' EXIT

export VERSION
perl -pe '
  # 1) 替换 [package]/[workspace.package] 里的 `version = "xxxx"`
  s{^\s*version\s*=\s*"[^"]*"}{version = "$ENV{VERSION}"};

  # 2) 替换 framework-xxx[-xxx] = { path = "...", version = "xxx" }
  s{^(framework-[A-Za-z0-9_\-]+)\s*=\s*\{\s*path\s*=\s*"([^"]+)"\s*,\s*version\s*=\s*"[^"]*"\s*\}}
   {$1 = { path = "$2", version = "$ENV{VERSION}" }};
' "$CARGO_TOML" > "$TMP_FILE"
mv "$TMP_FILE" "$CARGO_TOML"

echo "==> 已更新 Cargo.toml 版本号"

# ---------- 步骤 3: 只同步 workspace crate 到 Cargo.lock ----------
echo "==> 同步 Cargo.lock (仅 workspace crates, 不升级外部依赖)"
(
  cd "$ROOT_DIR"
  cargo update --workspace
)

# ---------- -d 模式直接退出 ----------
if $DRY_RUN; then
  echo "==> 已启用 -d, 跳过 git commit / tag / push"
  exit 0
fi

# ---------- 步骤 4: 提交 ----------
echo "==> git add . && git commit"
(
  cd "$ROOT_DIR"
  git add .
  git commit -m ":rocket: $VERSION"
)

# ---------- 步骤 5: 打 tag ----------
echo "==> 打 tag v$VERSION"
(
  cd "$ROOT_DIR"
  git tag "v$VERSION"
)

# ---------- 步骤 6: 推送到远程 ----------
echo "==> 推送到远程"
(
  cd "$ROOT_DIR"
  git push
  git push origin "v$VERSION"
)

echo "==> 完成: v$VERSION"