# External Diagram Workflow

## Problem

node-tikzjax has reliability issues:
- Race conditions in concurrent rendering
- Font loading failures
- UI runtime errors ("Cannot read properties of undefined")
- Limited TikZ feature support

## Solution: Generate Diagrams Externally

Generate SVG diagrams outside the build process and include them as static assets.

---

## Workflow

### Step 1: Create TikZ Source File

**File:** `docs/src/assets/diagrams/my-diagram.tex`

```latex
\documentclass[tikz,border=10pt]{standalone}
\usetikzlibrary{shapes.geometric,arrows.meta}

\begin{document}
\begin{tikzpicture}
  % Your TikZ code here
  \node[draw, circle] at (0,0) {A};
  \node[draw, circle] at (2,0) {B};
  \draw[->] (0,0) -- (2,0);
\end{tikzpicture}
\end{document}
```

### Step 2: Compile to SVG

**Option A: Using Overleaf (No Installation)**
1. Go to https://www.overleaf.com/
2. Create new blank project
3. Paste your `.tex` file
4. Compile (generates PDF)
5. Download PDF
6. Convert using: https://cloudconvert.com/pdf-to-svg

**Option B: Using Local LaTeX (If Installed)**
```bash
cd docs/src/assets/diagrams
pdflatex my-diagram.tex
pdf2svg my-diagram.pdf my-diagram.svg
```

**Option C: Using Docker**
```bash
docker run --rm -v $(pwd):/data texlive/texlive \
  pdflatex -output-directory=/data /data/my-diagram.tex

docker run --rm -v $(pwd):/data minidocks/pdf2svg \
  /data/my-diagram.pdf /data/my-diagram.svg
```

### Step 3: Include in MDX

```mdx
import MyDiagram from '../../assets/diagrams/my-diagram.svg?raw';

<figure class="diagram">
  <div dangerouslySetInnerHTML={{ __html: MyDiagram }} />
  <figcaption>My Diagram Caption</figcaption>
</figure>
```

Or use simple image tag:

```mdx
![My Diagram](../../assets/diagrams/my-diagram.svg)
```

---

## Advantages

✅ **Reliable**: No runtime errors or build failures
✅ **Full TikZ Support**: All libraries and features work
✅ **Fast Builds**: No compilation during build
✅ **Cacheable**: SVGs can be optimized and cached by CDN
✅ **Portable**: Works without node-tikzjax dependencies

---

## Current Recommendation

**For Minichain docs:**
- Keep simple diagrams as **ASCII** (readable in source, fast)
- Use **external SVG** for complex diagrams that need visual clarity
- **Disable** the remarkTikz plugin to avoid UI errors

---

## Disabling TikZ Plugin

**File:** `docs/astro.config.mjs`

```javascript
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
// import { remarkTikz } from './src/plugins/remark-tikz.ts'; // DISABLED

export default defineConfig({
  markdown: {
    remarkPlugins: [
      remarkMath,
      // remarkTikz, // DISABLED - Use external diagrams instead
    ],
    rehypePlugins: [[rehypeKatex, { /* ... */ }]],
  },
});
```

---

## Next Steps

1. **Decision**: Keep ASCII or migrate to external SVGs?
2. **If migrating**: Generate SVGs for high-value diagrams only
3. **Clean up**: Remove `.tikz-cache/` and remarkTikz plugin
4. **Document**: Add this workflow to contributor guide
