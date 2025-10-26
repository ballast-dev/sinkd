#!/bin/sh

echo "🟦 Bravo scenario starting..."

# Wait for sinkd client to be ready
sleep 10

# Create directories in bravo's local data
mkdir -p /data/bravo/files /data/bravo/common

echo "📁 Creating initial files in Bravo's local directory..."

# Create initial files
for i in $(seq 0 4); do
    echo "Initial content from bravo - file $i" > "/data/bravo/files/bravo_file_$i.txt"
    echo "✅ Created: /data/bravo/files/bravo_file_$i.txt"
    sleep 1
done

# Create shared document
echo "This is a shared document created by bravo
Line 2" > /data/bravo/common/shared_document.txt
echo "✅ Created shared file: /data/bravo/common/shared_document.txt"

# Periodic file creation
counter=0
while true; do
    sleep 10
    counter=$((counter + 1))
    timestamp=$(date +%s)
    
    # Create periodic file
    new_file="/data/bravo/files/bravo_periodic_$timestamp.txt"
    echo "Periodic file created by bravo at $timestamp" > "$new_file"
    echo "📝 Bravo created periodic file: $new_file"
    
    # Update shared document
    if [ -f "/data/bravo/common/shared_document.txt" ]; then
        echo "Bravo update at $timestamp" >> /data/bravo/common/shared_document.txt
        echo "📝 Bravo updated shared document"
    fi
done
