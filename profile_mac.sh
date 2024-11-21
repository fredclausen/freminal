#!/bin/bash

set -o errexit
set -o nounset

# verify we have xctrace
# xcrun -f xctrace is an error exit

# verify we have xctrace

if ! xcrun -f xctrace &> /dev/null
then
  echo "xctrace not found. Please install Xcode command line tools." 1>&2
  echo "Additionally, you may need to install Xcode" 1>&2
  echo "Additionally, you may need to run sudo xcode-select -r." 1>&2
  exit 1
fi


if [ "$#" -lt 1 ]
then
  echo "Usage $0 <program> [arguments...]" 1>&2
  exit 1
fi

PROGRAM="$(realpath "$1")"
shift

OUTPUT="/tmp/cpu_profile_$(whoami)_$(basename "$PROGRAM").trace"
echo "Profiling $PROGRAM into $OUTPUT" 1>&2
# Delete potential previous traces
rm -rf "$OUTPUT"
xcrun xctrace record \
  --template 'CPU Profiler' \
  --no-prompt \
  --output "$OUTPUT" \
  --target-stdout - \
  --launch -- "$PROGRAM" "$@"
open "$OUTPUT"
