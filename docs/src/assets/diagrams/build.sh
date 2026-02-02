#!/bin/bash
# Build TikZ diagrams to SVG
# Usage: ./build.sh [diagram-name]  (without .tex extension)
#        ./build.sh all              (build all .tex files)

set -e

build_diagram() {
    local name=$1
    echo "Building ${name}..."
    pdflatex -interaction=nonstopmode "${name}.tex" > /dev/null 2>&1
    pdf2svg "${name}.pdf" "${name}.svg"
    rm -f "${name}.pdf" "${name}.aux" "${name}.log"
    echo "✓ Created ${name}.svg"
}

if [ "$1" == "all" ]; then
    for tex_file in *.tex; do
        if [ -f "$tex_file" ]; then
            name="${tex_file%.tex}"
            build_diagram "$name"
        fi
    done
elif [ -n "$1" ]; then
    build_diagram "$1"
else
    echo "Usage: ./build.sh [diagram-name|all]"
    echo "Available diagrams:"
    ls -1 *.tex 2>/dev/null | sed 's/.tex$//' | sed 's/^/  /'
fi
