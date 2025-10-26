#!/bin/sh

echo "🟥 Alpha capture starting..."
echo "Will monitor /data/alpha/common/shared_document.txt and log changes to /workspace/scenario-output.log"

# Wait for shared document to be synced from bravo
sleep 15

output_log="/workspace/scenario-output.log"
prev_file="/tmp/prev_snapshot.txt"
curr_file="/tmp/curr_snapshot.txt"

# Initialize log file
echo "╔═══════════════════════════════════════════════════════════════╗" > "$output_log"
echo "║           SINKD SCENARIO OUTPUT - Alpha Capture               ║" >> "$output_log"
echo "║         Monitoring synced files via sinkd network             ║" >> "$output_log"
echo "║                   Showing line-by-line changes                ║" >> "$output_log"
echo "╚═══════════════════════════════════════════════════════════════╝" >> "$output_log"
echo "" >> "$output_log"

# Initialize with empty previous state
touch "$prev_file"

first_capture=true

while true; do
    sleep 5
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    if [ -f "/data/alpha/common/shared_document.txt" ]; then
        # Copy current file
        cp /data/alpha/common/shared_document.txt "$curr_file"
        
        # Check if file changed
        if ! cmp -s "$prev_file" "$curr_file"; then
            echo "" >> "$output_log"
            echo "┌───────────────────────────────────────────────────────────────┐" >> "$output_log"
            echo "│ 🔄 CHANGE DETECTED at: $timestamp              │" >> "$output_log"
            echo "└───────────────────────────────────────────────────────────────┘" >> "$output_log"
            echo "" >> "$output_log"
            
            if [ "$first_capture" = true ]; then
                echo "📄 Initial file state:" >> "$output_log"
                nl -ba "$curr_file" >> "$output_log"
                first_capture=false
            else
                echo "📝 Changes (+ added, - removed):" >> "$output_log"
                echo "" >> "$output_log"
                diff -u "$prev_file" "$curr_file" | tail -n +3 >> "$output_log" 2>/dev/null || {
                    echo "New lines added:" >> "$output_log"
                    nl -ba "$curr_file" >> "$output_log"
                }
            fi
            
            echo "" >> "$output_log"
            echo "═══════════════════════════════════════════════════════════════" >> "$output_log"
            
            # Update previous file
            cp "$curr_file" "$prev_file"
            
            echo "📸 Alpha detected changes at $timestamp"
        else
            echo "⏸️  No changes at $timestamp"
        fi
    else
        echo "⚠️  Shared document not synced yet, waiting for sinkd..."
    fi
done

