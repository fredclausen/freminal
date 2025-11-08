#!/usr/bin/env bash
set -euo pipefail

# ----------------------------
# Freminal Source Snapshot Tool
# ----------------------------
# Creates a clean zip of your Freminal repo for sharing or patch work.
# Example:
#   ./freminal_snapshot.sh ~/GitHub/freminal
# ----------------------------

# Target directory (defaults to ~/GitHub/freminal)
REPO_DIR="${1:-$HOME/GitHub/freminal}"

# Timestamped output name
TIMESTAMP="$(date +'%Y-%m-%dT%H-%M-%S')"
OUT_FILE="freminal_test_output_${TIMESTAMP}.txt"
OUTDIR="$REPO_DIR/snapshots"

echo "📦 Creating Freminal source snapshot"
echo "🗂  Repository: $REPO_DIR"
echo "🕒 Timestamp:  $TIMESTAMP"
echo "📄 Output:     $OUT_FILE"
echo

# ensure we're in the repo directory

cd "$REPO_DIR"

# make the output directory absolute for zip command

mkdir -p "$OUTDIR"

# Run tests and capture output

cargo llvm-cov &> "$OUTDIR/$OUT_FILE"

echo
echo "✅ Test Run Output: $REPO_DIR/$OUT_FILE"
echo
echo "You can now upload $OUT_FILE to ChatGPT for patching."
