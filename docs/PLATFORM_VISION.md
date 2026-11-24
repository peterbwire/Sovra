# Sovra platform goal

Sovra is a general-purpose language and software ecosystem for humans and AI
agents to build applications and systems across mobile, desktop, web, servers
and other environments. This is the long-term product goal, reaffirmed by the
user on 2026-11-11. Compiler, runtime, UI, library and developer-tool work should
advance that goal incrementally.

## Intended targets

| Target | Intended application environments |
| --- | --- |
| Mobile | Android and iOS applications |
| Desktop | Windows, macOS and Linux applications |
| Web | Browser applications and web services |
| Servers and command line | Backend services, tools and automation |
| Other environments | Additional targets as their runtime and deployment needs are defined |

These are intended targets, not a support matrix. Current implementation and
verification belong in [full development status](FULL_DEVELOPMENT_STATUS.md),
the [roadmap](roadmap.md) and individual backend/host documents. Mobile runtime,
UI and packaging support remains planned. A working JavaScript backend does not
by itself establish browser support; a native host build does not establish
cross-compilation or mobile deployment.

## Direction for implementation

- Keep language semantics, the checked compiler representation, diagnostics and
  reusable libraries independent of a particular OS, UI toolkit or IDE.
- Put platform-specific rendering, device services, lifecycle, filesystem/process
  integration and packaging behind explicit backend or host boundaries. Document
  which capabilities each target provides and how unavailable capabilities fail.
- Share application logic where its dependencies permit it. Do not promise that
  every program or host API works unchanged on every target.
- Preserve equivalent language behavior across supported execution engines and
  qualify target-specific permissions, resource limits and application lifecycle
  through actual target tests. Security guarantees must have evidence per target.
- Use the Task Manager, Windows desktop host, browser editor and VS Code extension
  as implementation and validation slices. They do not define the language's
  full purpose or restrict its future targets.

This direction does not choose a mobile framework, backend technology, UI ABI,
new language syntax or memory model. Make those decisions through the existing
architecture process when implementation requires them. Existing milestone
numbers and release qualification gates remain in place; shipping a focused
release is progress toward the broader goal, not completion of all targets.
