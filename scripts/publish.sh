#!/usr/bin/env bash

set -euo pipefail

# 项目根目录
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TOML="${ROOT_DIR}/Cargo.toml"

if [[ ! -f "${CARGO_TOML}" ]]; then
    echo "错误: 未找到 ${CARGO_TOML}" >&2
    exit 1
fi

cd "${ROOT_DIR}"

# 读取 version 开头和 framework 开头的行
mapfile -t VERSION_LINES < <(
    grep -E '^[[:space:]]*(version|framework)' "${CARGO_TOML}" || true
)

if [[ ${#VERSION_LINES[@]} -eq 0 ]]; then
    echo "错误: ${CARGO_TOML} 中未找到 version/framework 配置" >&2
    exit 1
fi

# 提取 workspace/package 的基准 version
BASE_VERSION=""

for line in "${VERSION_LINES[@]}"; do
    if [[ "${line}" =~ ^[[:space:]]*version[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
        if [[ -n "${BASE_VERSION}" ]]; then
            echo "错误: ${CARGO_TOML} 中发现多个 version 基准版本:" >&2
            echo "  已发现: ${BASE_VERSION}" >&2
            echo "  当前:   ${BASH_REMATCH[1]}" >&2
            exit 1
        fi

        BASE_VERSION="${BASH_REMATCH[1]}"
    fi
done

if [[ -z "${BASE_VERSION}" ]]; then
    echo "错误: ${CARGO_TOML} 中未找到以 version 开头的基准版本" >&2
    exit 1
fi

echo "基准版本: ${BASE_VERSION}"

# 检查 framework 开头行中的 version
HAS_MISMATCH=0

for line in "${VERSION_LINES[@]}"; do
    if [[ "${line}" =~ ^[[:space:]]*framework[^=]*=.*version[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
        FRAMEWORK_VERSION="${BASH_REMATCH[1]}"

        if [[ "${FRAMEWORK_VERSION}" != "${BASE_VERSION}" ]]; then
            echo "版本不一致:" >&2
            echo "  来源: ${line}" >&2
            echo "  基准版本: ${BASE_VERSION}" >&2
            echo "  实际版本: ${FRAMEWORK_VERSION}" >&2
            echo >&2

            HAS_MISMATCH=1
        fi
    fi
done

if [[ "${HAS_MISMATCH}" -ne 0 ]]; then
    echo "错误: framework 依赖版本与基准版本不一致，取消发布。" >&2
    exit 1
fi

echo "版本检查通过: 所有 framework 版本均为 ${BASE_VERSION}"
echo "开始发布..."

# 对新crate 有 每分钟5个的并发限制, 所以要加发布间隔
cargo workspaces publish --publish-as-is --locked --yes --no-git-commit --publish-interval 15
