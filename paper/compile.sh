#!/usr/bin/env bash
# compile.sh — Build paper PDF from LaTeX source
# Requires: pdflatex (preferred) or pandoc (fallback)
set -euo pipefail
cd "$(dirname "$0")"

if command -v pdflatex &>/dev/null; then
  echo "✓ Compiling with pdflatex ..."
  pdflatex -interaction=nonstopmode ed2kIA_sae_audit.tex
  pdflatex -interaction=nonstopmode ed2kIA_sae_audit.tex
  echo "✓ → ed2kIA_sae_audit.pdf"
elif command -v pandoc &>/dev/null; then
  echo "✓ Compiling with pandoc (fallback) ..."
  pandoc ed2kIA_sae_audit.tex -o ed2kIA_sae_audit.pdf
  echo "✓ → ed2kIA_sae_audit.pdf"
else
  echo "✗ Neither pdflatex nor pandoc found."
  echo "  Install TeX Live (Ubuntu: sudo apt install texlive-latex-recommended)"
  echo "  or Pandoc (https://pandoc.org/installing.html)"
  exit 1
fi
