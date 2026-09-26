# Sovra website

A responsive, dependency-free static website for the Sovra programming language.
Includes real course/repository links, source installation instructions, a
downloadable executable example, copy buttons, and accessible mobile navigation.

## Run locally

With Node.js 22 or newer, from the repository root:

```text
cd sovra-website
node server.mjs
```

Open http://127.0.0.1:4173. No npm install is needed. Set `PORT` to choose
another port. The development server binds only to your computer.

You can also open `index.html` directly. If clipboard access is unavailable,
copy buttons select the text for manual copying. Navigation and content remain
available with JavaScript disabled.

## Verify

```text
node --test tests/site.test.mjs
```

`npm start` and `npm test` are also available where the shell permits npm's
launcher. The direct Node commands above require no shell policy changes.

Tests check local assets, anchor targets, repository document paths, example
consistency, HTTP responses, navigation state, and clipboard success/failure.
Interaction tests use a small DOM fixture; they do not replace visual browser checks.
External links use the configured Git origin, `https://github.com/peterbwire/Sovra`.
Document links follow its default branch; tests verify those paths locally,
not GitHub availability.

To verify the downloadable example with the compiler, from the repository root:

```text
cargo run -- run sovra-website/examples/hello.svr
```

Expected output: `Hello, Ada`.

## Publish

Upload `index.html`, `styles.css`, `script.js`, and `examples/hello.svr` together
to any static host, preserving their relative paths. No build step, backend,
accounts, or environment secrets are required. The development server is not
needed on the static host. Publishing is separate from local setup.

## Files

- `index.html` — site content and structure
- `styles.css` — responsive dark developer-focused design
- `script.js` — navigation, copy controls, and footer year
- `examples/hello.svr` — downloadable source matching the hero example
- `server.mjs` — local preview server
- `tests/site.test.mjs` — functional checks
