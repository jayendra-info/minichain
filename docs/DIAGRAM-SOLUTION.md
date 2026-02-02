# Diagram Solution - External TikZ Compilation

## Problem

node-tikzjax build-time rendering was causing:
- Race conditions (7% success rate initially)
- Font loading errors (`tcrm1000` not found)
- UI runtime errors ("Cannot read properties of undefined")
- Complex debugging and unreliable builds

## Solution

**Compile TikZ diagrams externally** using full LaTeX installation, then import SVGs as static assets.

### Advantages

✅ **Reliable** - Full LaTeX support, no runtime errors
✅ **Fast builds** - No compilation during build process
✅ **Full features** - All TikZ libraries and features work
✅ **Optimized** - Astro processes and hashes SVGs automatically
✅ **Simple** - One script to compile all diagrams

## Implementation

### 1. Directory Structure

```
docs/src/assets/diagrams/
├── README.md           # Documentation
├── build.sh            # Compilation script
├── decision-tree.tex   # TikZ source
├── decision-tree.svg   # Compiled output
├── ecc-timeline.tex    # TikZ source
└── ecc-timeline.svg    # Compiled output
```

### 2. Build Script

```bash
./build.sh diagram-name  # Compile one
./build.sh all           # Compile all
```

**What it does:**
1. Runs `pdflatex` on `.tex` file
2. Converts PDF to SVG with `pdf2svg`
3. Cleans up auxiliary files
4. Reports success

### 3. Usage in MDX

```mdx
![Diagram Title](../../../assets/diagrams/diagram-name.svg)
```

Astro automatically:
- Optimizes SVG
- Adds content hash to filename
- Copies to `dist/_astro/` folder
- Updates references

## Current Diagrams

### Decision Tree
**File:** `decision-tree.svg`
**Description:** Visual guide for choosing elliptic curves
**Features:**
- Ellipse question node
- Colored answer boxes (blue, green, purple, orange)
- Labeled arrows with orthogonal routing

### ECC Timeline
**File:** `ecc-timeline.svg`
**Description:** History of elliptic curve cryptography
**Features:**
- Vertical timeline with year markers
- Era boxes highlighting periods
- Event descriptions with details

## Prerequisites

```bash
sudo apt install -y texlive-latex-base texlive-latex-extra texlive-pictures pdf2svg
```

## Configuration Changes

### Disabled
- ❌ `remarkTikz` plugin in `astro.config.mjs`
- ❌ node-tikzjax dependency
- ❌ `.tikz-cache/` directory

### Kept
- ✅ KaTeX math rendering (works perfectly)
- ✅ Custom CSS for styling
- ✅ Markdown processing pipeline

## Migration Strategy

### Migrate to TikZ SVG
Best for:
- **Decision trees** - Simple branching structures
- **Timelines** - Linear chronological flows
- **Taxonomies** - Hierarchical classifications
- **Architecture diagrams** - System overviews

**Criteria:**
- Visual clarity matters
- 5-10 nodes maximum
- Relatively simple structure
- Worth the effort

### Keep as ASCII
Best for:
- **Algorithm flows** - Step-by-step processes
- **Code annotations** - Inline explanations
- **Simple structures** - Lists, basic trees
- **Text-heavy diagrams** - Multi-column layouts

**Criteria:**
- Quick to write/edit
- Readable in source
- No special visual needs
- Not worth LaTeX complexity

## Results

### Build Performance
- **Clean build:** ~12 seconds (11 pages)
- **Incremental:** ~1-2 seconds
- **No TikZ compilation** during build
- **No errors or warnings**

### Diagrams Created
- ✅ 2 diagrams migrated
- ✅ Both rendering correctly
- ✅ Astro optimization working
- ✅ Accessible in documentation

### Remaining ASCII Diagrams
- 6 diagrams in appendix
- Kept intentionally (see migration strategy)
- Can be migrated individually if needed

## Commands Reference

### Compile Diagrams
```bash
cd docs/src/assets/diagrams
./build.sh decision-tree  # Single diagram
./build.sh all            # All diagrams
```

### Build Documentation
```bash
cd docs
bun run build             # Full build
bun run dev               # Development server
```

### Verify Output
```bash
# Check compiled SVGs exist
ls -lh src/assets/diagrams/*.svg

# Check optimized SVGs in dist
find dist/_astro -name "*.svg"

# View in browser
bun run dev
# Navigate to: http://localhost:4321/appendix/elliptic-curves
```

## Maintenance

### Adding New Diagrams

1. **Create `.tex` file** in `diagrams/` directory
2. **Compile:** `./build.sh diagram-name`
3. **Import in MDX:** `![Title](../../../assets/diagrams/diagram-name.svg)`
4. **Build docs:** `bun run build`
5. **Verify:** Check browser

### Updating Existing Diagrams

1. **Edit `.tex` file**
2. **Recompile:** `./build.sh diagram-name`
3. **Rebuild docs:** `bun run build`
4. **Hard refresh browser** (Ctrl+F5)

### Troubleshooting

**SVG not updating?**
```bash
rm -rf dist
bun run build
```

**LaTeX errors?**
```bash
pdflatex diagram-name.tex  # See full error output
```

**Wrong path in MDX?**
- Verify: `../../../assets/diagrams/filename.svg`
- Count directories from MDX file location

## Documentation Files

- `DIAGRAM-SOLUTION.md` - This file (overview)
- `EXTERNAL-DIAGRAM-WORKFLOW.md` - Detailed workflow guide
- `diagrams/README.md` - Quick reference
- `diagrams/build.sh` - Compilation script

## Success Criteria

✅ **Reliability:** No build failures or UI errors
✅ **Quality:** Professional-looking diagrams
✅ **Performance:** Fast builds (<15 seconds)
✅ **Maintainability:** Simple workflow documented
✅ **Flexibility:** Can use all TikZ features

## Next Steps

1. **Optional:** Migrate more ASCII diagrams if valuable
2. **Optional:** Add more diagram types (flowcharts, etc.)
3. **Focus:** Content writing with working infrastructure

## Lessons Learned

1. **Build-time rendering is hard** - Libraries like node-tikzjax have limitations
2. **External compilation is reliable** - Full LaTeX toolchain just works
3. **One-time cost is acceptable** - Diagram changes are infrequent
4. **Static assets are simple** - No complex plugins needed
5. **ASCII still has value** - Not everything needs visual polish

---

**Status:** Production-ready ✅

**Build:** Passing ✅

**Diagrams:** 2 migrated, working ✅
