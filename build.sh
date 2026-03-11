#!/bin/bash

# Exit immediately if any command fails
set -e

echo "⚙️  Compiling Rust DSP Engine..."
cd tuner-dsp
wasm-pack build --target web
cd ..

echo "🧹 Preparing production distribution..."
# Ensure the dist/pkg directory exists and is clean
rm -rf dist/pkg
mkdir -p dist/pkg

echo "📦 Extracting Wasm payload (Bypassing .gitignore)..."
# Your exact, brilliant extraction logic
cp tuner-dsp/pkg/tuner_dsp* dist/pkg/
cp tuner-dsp/pkg/package.json dist/pkg/

# Optional: Copy your web files just in case you update them
cp web/index.html dist/
cp web/*.js dist/

echo "✅ Build complete! The 'dist/' folder is perfectly staged for Vercel."
