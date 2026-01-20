# Minichain Documentation

Documentation for "Building a Blockchain from Scratch" - a hands-on guide to building a minimal blockchain with Rust.

Built with [Astro Starlight](https://starlight.astro.build).

## Project Structure

```
docs/
├── public/              # Static assets (favicon, images)
├── src/
│   ├── assets/          # Images embedded in markdown
│   ├── content/
│   │   └── docs/        # Documentation pages (.md/.mdx)
│   └── styles/          # Custom CSS
├── astro.config.mjs     # Astro configuration
├── Dockerfile           # Docker setup for serving
└── package.json
```

## Development

### Prerequisites

- [Bun](https://bun.sh/) (recommended) or Node.js 18+

### Commands

| Command        | Action                                      |
| :------------- | :------------------------------------------ |
| `bun install`  | Install dependencies                        |
| `bun dev`      | Start local dev server at `localhost:4321`  |
| `bun build`    | Build production site to `./dist/`          |
| `bun preview`  | Preview build locally before deploying      |

## Docker

Build and run the documentation using Docker:

```bash
# Build the image
docker build -t minichain-docs .

# Run the container
docker run -p 8080:80 minichain-docs
```

The documentation will be available at `http://localhost:8080`.

## Content

The book covers:

- **Part 1: Foundation** - Core primitives (hashing, cryptography, accounts, transactions, blocks)
- **Part 2: Storage** - Persistent state with RocksDB
- **Part 3: Virtual Machine** - Register-based VM with gas metering
- **Part 4: Assembler** - Assembly language to bytecode compiler
- **Part 5: Blockchain** - Consensus and chain management
- **Part 6: CLI** - Command line interface and REPL

## Adding Content

1. Create `.md` or `.mdx` files in `src/content/docs/`
2. Each file becomes a route based on its path
3. Update `astro.config.mjs` sidebar configuration if needed
