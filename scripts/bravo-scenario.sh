#!/bin/sh

echo "🟦 Bravo scenario starting..."

# Wait for sinkd client to be ready
sleep 10

echo "📝 Bravo modifying shared test file..."

# Wait a bit more to ensure sync is ready
sleep 5

# Modify the shared test file
timestamp=$(date +%s)
echo "--- BRAVO MODIFICATION ---" >> /workspace/test/shared_test_file.txt
echo "Bravo was here! Modified at timestamp: $timestamp" >> /workspace/test/shared_test_file.txt

echo "✅ Bravo completed modification of shared_test_file.txt"

# Keep container running
sleep infinity
