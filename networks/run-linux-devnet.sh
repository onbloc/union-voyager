#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
system="${DEVNET_LINUX_SYSTEM:-aarch64-linux}"
target="${DEVNET_TARGET:-full-dev-setup}"
no_blockscout="${NO_BLOCKSCOUT:-true}"

case "$system" in
  aarch64-linux)
    platform="linux/arm64/v8"
    volume="union-nix-store-aarch64-linux"
    ;;
  x86_64-linux)
    platform="linux/amd64"
    volume="union-nix-store-x86_64-linux"
    ;;
  *)
    echo "unsupported DEVNET_LINUX_SYSTEM=$system; use aarch64-linux or x86_64-linux" >&2
    exit 2
    ;;
esac

docker_args=(--rm)
if [ -t 0 ] && [ -t 1 ]; then
  docker_args+=(-it)
fi

exec docker run "${docker_args[@]}" \
  --platform "$platform" \
  -v "$repo_root:/work/union" \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v "$volume:/nix" \
  -w /work/union \
  -e TARGET="$target" \
  -e NO_BLOCKSCOUT="$no_blockscout" \
  -e NIX_CONFIG="experimental-features = nix-command flakes" \
  nixos/nix:latest \
  sh -lc 'nix --accept-flake-config shell nixpkgs#docker-client nixpkgs#git -c sh -lc '"'"'
    git config --global --add safe.directory /work/union
    nix --accept-flake-config run ".#$TARGET"
  '"'"''
