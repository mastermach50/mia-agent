#!/bin/env sh
nix build .#x86_64-linux   -o all_builds/x86_64-linux                 823ms  ~/Documents/Code/Rust/mia-agent
nix build .#aarch64-linux  -o all_builds/aarch64-linux
nix build .#x86_64-windows -o all_builds/x86_64-windows
nix build .#aarch64-windows -o all_builds/aarch64-windows