#!/bin/bash
# IVSG Launcher for WSL2
# Tries multiple graphics backends

echo "Attempting to launch IVSG with WSL2-compatible graphics..."

# Method 1: Software OpenGL
echo "Trying software OpenGL rendering..."
LIBGL_ALWAYS_SOFTWARE=1 target/debug/ivsg &
PID=$!
sleep 2

if kill -0 $PID 2>/dev/null; then
    echo "✓ Running with software OpenGL (PID: $PID)"
    wait $PID
    exit 0
else
    echo "✗ Software OpenGL failed"
fi

# Method 2: Vulkan with lavapipe
echo "Trying Vulkan software rendering..."
WGPU_BACKEND=vulkan VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json target/debug/ivsg &
PID=$!
sleep 2

if kill -0 $PID 2>/dev/null; then
    echo "✓ Running with Vulkan lavapipe (PID: $PID)"
    wait $PID
    exit 0
else
    echo "✗ Vulkan failed"
fi

# Method 3: Default (may show errors but might work)
echo "Trying default renderer..."
target/debug/ivsg

