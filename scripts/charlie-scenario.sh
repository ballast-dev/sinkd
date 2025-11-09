#!/bin/sh

echo "🟨 Charlie scenario starting..."

# Wait for sinkd client to be ready and for bravo's modification to sync
sleep 30

echo "📝 Charlie modifying shared test file after Bravo..."

# Check if bravo's modification is present (wait for sync)
max_wait=60
waited=0
while [ $waited -lt $max_wait ]; do
    if grep -q "BRAVO MODIFICATION" /workspace/test/shared_test_file.txt; then
        echo "✅ Bravo's modification detected, Charlie proceeding..."
        break
    fi
    echo "⏳ Waiting for Bravo's modification to sync... ($waited/$max_wait)"
    sleep 5
    waited=$((waited + 5))
done

# Modify the shared test file after bravo
timestamp=$(date +%s)
echo "--- CHARLIE MODIFICATION ---" >> /workspace/test/shared_test_file.txt
echo "Charlie was here after Bravo! Modified at timestamp: $timestamp" >> /workspace/test/shared_test_file.txt

echo "✅ Charlie completed modification of shared_test_file.txt"

# Keep container running
sleep infinity
