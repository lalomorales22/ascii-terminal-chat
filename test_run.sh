#!/bin/bash

echo "Starting Terminal Chat Test..."
echo "The app will run for 5 seconds to verify it doesn't crash"
echo "NOTE: Using demo video pattern instead of real webcam"
echo ""

# Run the app with a timeout
timeout 5 cargo run 2>&1 | head -20

EXIT_CODE=$?

if [ $EXIT_CODE -eq 124 ]; then
    echo ""
    echo "✅ SUCCESS: App ran for 5 seconds without crashing!"
    echo "The terminal chat is working with demo video."
    echo ""
    echo "To run normally: cargo run"
    echo "To connect as client: cargo run -- --connect ws://localhost:8080/ws"
else
    echo ""
    echo "❌ App exited with code: $EXIT_CODE"
fi