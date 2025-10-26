#!/bin/sh

echo "🟨 Charlie scenario starting..."

# Wait for sinkd client to be ready and for files to sync from bravo
sleep 20

# Create charlie's directory in charlie's local data
mkdir -p /data/charlie/files /data/charlie/common

counter=0
while true; do
    sleep 15
    counter=$((counter + 1))
    timestamp=$(date +%s)
    
    # Create charlie's own files
    charlie_file="/data/charlie/files/charlie_response_$timestamp.txt"
    echo "Charlie's response file created at $timestamp" > "$charlie_file"
    echo "📝 Charlie created: $charlie_file"
    
    # Modify bravo's files if they exist (synced via sinkd)
    for file in /data/charlie/files/bravo_file_*.txt; do
        if [ -f "$file" ]; then
            echo "
--- Modified by Charlie at $timestamp ---" >> "$file"
            echo "✏️  Charlie modified: $file"
            break  # Only modify one file per cycle
        fi
    done
    
    # Update shared document (synced via sinkd)
    if [ -f "/data/charlie/common/shared_document.txt" ]; then
        echo "Charlie's contribution at $timestamp" >> /data/charlie/common/shared_document.txt
        echo "📝 Charlie updated shared document"
    fi
done
