#/usr/bin/env sh
find . -path "./target" -prune -o -name "*.rs" -print0 | xargs -0 wc -l
