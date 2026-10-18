#!/usr/bin/env sh
# SPDX-License-Identifier: AGPL-3.0-only
# Fails if the dependency tree of the headless CLI contains GUI, windowing or accessibility-toolkit crates
# (ARCHITECTURE: `cargo build -p armasync-cli --locked` must work without GUI dependencies).
set -eu
tree="$(cargo tree -p armasync-cli --locked -e normal --prefix none --target all)"
if printf '%s\n' "$tree" | grep -E '^(armasync-gui|eframe|egui|egui[-_a-z0-9]*|winit|accesskit[-_a-z0-9]*|wgpu[-_a-z0-9]*|glow|glutin[-_a-z0-9]*|raw-window-handle|wayland-[a-z-]+|x11[-a-z0-9]*|xkbcommon[-a-z0-9]*) '; then
  echo "armasync-cli depends on GUI/windowing crates (listed above)" >&2
  exit 1
fi
echo "armasync-cli dependency tree is GUI-free ($(printf '%s\n' "$tree" | sort -u | wc -l | tr -d ' ') distinct crates)"
