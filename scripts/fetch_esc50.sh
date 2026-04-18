#!/usr/bin/env bash
# Download ESC-50 dataset into benchmark_audio/ for real-accuracy eval.
# ~250 MB download, CC-BY-NC licensed.
#
# Idempotent: re-running is safe.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="${ROOT}/benchmark_audio"
CACHE="${ROOT}/.cache/esc50"

mkdir -p "${WORK}" "${CACHE}"

if [ ! -f "${CACHE}/esc50.zip" ]; then
    echo "Downloading ESC-50 (~250 MB)..."
    curl -L -o "${CACHE}/esc50.zip" \
        "https://github.com/karolpiczak/ESC-50/archive/master.zip"
fi

if [ ! -d "${CACHE}/ESC-50-master" ]; then
    echo "Extracting..."
    unzip -q -o "${CACHE}/esc50.zip" -d "${CACHE}"
fi

# Copy wav files into benchmark_audio/ (flat)
if [ -z "$(ls -A "${WORK}" 2>/dev/null | grep -E '\.wav$' | head -1)" ]; then
    echo "Copying audio files to benchmark_audio/..."
    cp "${CACHE}/ESC-50-master/audio/"*.wav "${WORK}/"
    cp "${CACHE}/ESC-50-master/meta/esc50.csv" "${WORK}/esc50_meta.csv"
fi

echo "✓ ESC-50 ready: $(ls "${WORK}"/*.wav | wc -l) clips in ${WORK}"
