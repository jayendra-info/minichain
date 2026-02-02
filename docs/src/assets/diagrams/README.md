# TikZ Diagrams

This directory contains TikZ source files (`.tex`) and their compiled SVG outputs.

## Why External Compilation?

node-tikzjax (build-time TikZ rendering) proved unreliable with:
- Race conditions
- Font loading errors
- UI runtime errors
- Limited library support

**Solution:** Compile diagrams externally with full LaTeX, then import SVGs.

## Workflow

### 1. Create Diagram

Create a new `.tex` file:

```latex
\documentclass[tikz,border=10pt]{standalone}
\usetikzlibrary{shapes.geometric}

\begin{document}
\begin{tikzpicture}
  \node[draw, circle] at (0,0) {Example};
\end{tikzpicture}
\end{document}
```

### 2. Compile to SVG

```bash
./build.sh my-diagram
# or compile all:
./build.sh all
```

This runs:
1. `pdflatex my-diagram.tex` → PDF
2. `pdf2svg my-diagram.pdf my-diagram.svg` → SVG
3. Cleanup auxiliary files

### 3. Import in MDX

```mdx
![Diagram Title](../../../assets/diagrams/my-diagram.svg)
```

Astro will automatically optimize and hash the SVG during build.

## Prerequisites

Install LaTeX tools:

```bash
sudo apt install -y texlive-latex-base texlive-latex-extra texlive-pictures pdf2svg
```

Verify:

```bash
pdflatex --version
pdf2svg -v
```

## Available Diagrams

- `decision-tree.svg` - Elliptic curve selection guide
- `ecc-timeline.svg` - History of ECC development

## Tips

1. **Use absolute positioning** - `\node at (x,y)` instead of relative `below=of`
2. **Keep simple** - 5-10 nodes max for clarity
3. **Test incremental** - Build and check after each addition
4. **Border matters** - `border=10pt` gives nice padding
5. **Libraries** - Common ones that work:
   - `shapes.geometric` - circles, ellipses, diamonds
   - `arrows.meta` - arrow styles
   - Basic positioning (absolute coordinates)

## Troubleshooting

### PDF compiles but looks wrong?
- Check node positions (absolute coordinates)
- Verify library imports at top
- Try with minimal example first

### SVG not showing in docs?
- Verify path: `../../../assets/diagrams/filename.svg`
- Check file exists in `dist/_astro/` after build
- Clear cache: `rm -rf dist && bun run build`

### LaTeX errors?
- Run `pdflatex diagram.tex` manually to see full errors
- Check TikZ syntax
- Verify all `\begin` have matching `\end`

## References

- [TikZ Manual](https://tikz.dev/)
- [Overleaf TikZ Gallery](https://www.overleaf.com/learn/latex/TikZ_package)
- Main docs: `/docs/EXTERNAL-DIAGRAM-WORKFLOW.md`
