#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
system=${DEVNET_LINUX_SYSTEM:-aarch64-linux}
target=${1:?usage: run-linux-nix.sh FLAKE_TARGET [ARG ...]}
shift

[[ $target =~ ^[a-zA-Z0-9._-]+$ ]] || { echo "invalid flake target: $target" >&2; exit 2; }

case "$system" in
  aarch64-linux)
    platform=linux/arm64/v8
    volume=union-nix-store-aarch64-linux
    ;;
  x86_64-linux)
    platform=linux/amd64
    volume=union-nix-store-x86_64-linux
    ;;
  *)
    echo "unsupported DEVNET_LINUX_SYSTEM=$system; use aarch64-linux or x86_64-linux" >&2
    exit 2
    ;;
esac
cache_volume="union-nix-cache-$system"

exec docker run --rm \
  --platform "$platform" \
  --network host \
  -v "$repo_root:/work/union" \
  -v "$volume:/nix" \
  -v "$cache_volume:/root/.cache/nix" \
  -w /work/union \
  -e TARGET="$target" \
  -e NIX_CONFIG="experimental-features = nix-command flakes" \
  nixos/nix:latest \
  sh -lc '
    nix --accept-flake-config shell nixpkgs#git -c bash -c '\''
      git config --global --add safe.directory /work/union
      nix --accept-flake-config run --impure ".#$TARGET" -- "$@"
    '\'' union-linux-nix "$@"
  ' union-linux-nix "$@"
