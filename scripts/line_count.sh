#!/usr/bin/env bash

DIR="${1:-.}"

RS_TOTAL=0

echo "=== .rs files ==="

while IFS= read -r -d '' FILE; do
    COUNT=$(wc -l < "$FILE")
    RS_TOTAL=$((RS_TOTAL + COUNT))
    printf "%6d %s\n" "$COUNT" "$FILE"
done < <(
    find "$DIR" \( \
        -name target -o \
        -name .git -o \
        -name .flatpak-builder \) -prune -false \
        -o -name '*.rs' -print0
)

echo
echo "Total .rs lines:   $RS_TOTAL"
