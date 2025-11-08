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
OUT_FILE="freminal_snapshot_${TIMESTAMP}.zip"

echo "📦 Creating Freminal source snapshot"
echo "🗂  Repository: $REPO_DIR"
echo "🕒 Timestamp:  $TIMESTAMP"
echo "📄 Output:     $OUT_FILE"
echo

# Verify directory
if [[ ! -d "$REPO_DIR" ]]; then
  echo "❌ Error: Directory '$REPO_DIR' does not exist."
  exit 1
fi

cd "$REPO_DIR"

# Exclude patterns (safe defaults)
EXCLUDES=(
  '*.git*'
  'target/*'
  '*/target/*'
  'result'
  'result/*'
  'dist/*'
  'Cargo.lock'
  '*.DS_Store'
  'flake.lock'
  '.direnv/*'
  '.envrc'
  'node_modules/*'
  '*.old'
  '*.bak'
  'res/*'
  'Documents/*'
  'speed_tests/*'
)

# Construct zip exclude args
EXCLUDE_ARGS=()
for pattern in "${EXCLUDES[@]}"; do
  EXCLUDE_ARGS+=(-not -path "$pattern")
done

echo "🔍 Listing files to include..."
# List included files using find, applying exclusion rules
# shellcheck disable=SC2046
find . -type f \
  -not -path '*/\.*' \
  $(for e in "${EXCLUDES[@]}"; do echo "-not -path '$e'"; done) \
  | sort | tee /tmp/freminal_snapshot_filelist.txt &> /dev/null

echo
echo "📄 Total files to be archived: $(wc -l < /tmp/freminal_snapshot_filelist.txt)"
echo "⚙️  Building archive..."
echo

# Use zip with exclusion patterns
zip -r "$OUT_FILE" . \
  -x "${EXCLUDES[@]}" >/dev/null

echo
echo "✅ Snapshot complete: $REPO_DIR/$OUT_FILE"
echo "   Total files included: $(wc -l < /tmp/freminal_snapshot_filelist.txt)"
echo
echo "You can now upload $OUT_FILE to ChatGPT for patching."
