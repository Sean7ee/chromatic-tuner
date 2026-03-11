#!/bin/bash

echo "⚙️  Compiling Rust DSP engine..."

# 1. Navigate to the DSP crate and build
cd tuner-dsp || exit
wasm-pack build --target web

# 2. Catch compilation errors before starting the server
if [ $? -ne 0 ]; then
    echo "❌ Wasm build failed. Check your Rust code."
    # Return to root directory before exiting
    cd .. 
    exit 1
fi

echo "✅ Build successful!"

# 3. Navigate back to the workspace root
cd ..

echo "🚀 Starting local server..."
echo "👉 Open your browser to: http://localhost:8000/web/"

# 4. Spin up the server
python -m http.server 8000
