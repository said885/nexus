# Contributing to NEXUS

Thank you for your interest in contributing to NEXUS! We believe great security software is built by communities. Whether you're fixing a bug, adding a feature, or improving documentation, your contribution matters.

## Contributor License Agreement (CLA)

Before your first contribution can be merged, you must sign the
[Contributor License Agreement](CLA.md). This protects both you and the
project by ensuring intellectual property rights are clearly defined.

Include the following in your pull request description:

> I have read the NEXUS Contributor License Agreement and I agree to its
> terms. My contributions are my original work and I have the right to
> submit them under the project license.

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md) in all interactions.

## Getting Started

### Prerequisites

- [Rust 1.75+](https://rustup.rs/)
- [Node.js 20+](https://nodejs.org/)
- [Git](https://git-scm.com/)
- [Docker & Docker Compose](https://docs.docker.com/get-docker/) (for testing the full stack)

### Clone & Setup

```bash
git clone https://github.com/nexus-project/nexus.git
cd nexus

# Set up development environment
cp nexus-relay/.env.example nexus-relay/.env

# Run tests
cd nexus-relay
cargo test

# Run linter (MUST pass with 0 warnings)
cargo clippy --all-targets -- -D warnings

# Run formatter
cargo fmt --all
```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feat/your-feature-name
# or
git checkout -b fix/your-bug-fix
# or
git checkout -b docs/your-documentation
```

**Branch naming conventions:**
- `feat/` — New features
- `fix/` — Bug fixes
- `docs/` — Documentation updates
- `perf/` — Performance improvements
- `refactor/` — Code refactorings (no logic changes)
- `test/` — Tests and test infrastructure
- `chore/` — Dependency updates, config changes

### 2. Make Changes

Follow these guidelines:

#### Rust Code

- **0 Warnings** — Run `cargo clippy -- -D warnings` before committing
- **Formatted** — Run `cargo fmt --all` to auto-format
- **Tested** — Add tests for new functionality
- **Documented** — Add doc comments for public APIs
- **Safe** — Avoid `unsafe` unless absolutely necessary (and explain why)

Example:

```rust
/// Encrypts a message using the session key.
///
/// # Arguments
/// * `message` - The plaintext message to encrypt
/// * `session_key` - The 32-byte session key
///
/// # Returns
/// * `Result<Vec<u8>>` - Ciphertext or error
///
/// # Example
/// ```
/// let ct = encrypt_message(b"hello", &key)?;
/// ```
pub fn encrypt_message(message: &[u8], session_key: &[u8; 32]) -> Result<Vec<u8>> {
    // Implementation
}
```

#### Tests

- Write tests in the same file as the code (unit tests)
- Write integration tests in `tests/` directory
- Aim for >80% code coverage for security-critical code

```bash
cargo test
cargo tarpaulin --out Html  # Generate coverage report
```

#### TypeScript/React Code

- Use **Prettier** for formatting: `npm run format`
- Use **ESLint**: `npm run lint`
- Add **JSDoc** comments for exported functions
- Follow **React** best practices

### 3. Commit Your Changes

Use clear, descriptive commit messages:

```bash
git commit -m "feat: add hybrid KEM key agreement

- Implement Kyber1024 + X25519 hybrid KEM
- Add 150 unit tests for key exchange
- Update threat model documentation
- Verified via formal TLA+ model

Fixes #123
```

**Commit message format:**
```
<type>: <subject>

<body>

<footer>
```

- **type**: `feat`, `fix`, `docs`, `test`, `perf`, `refactor`, `chore`
- **subject**: < 50 characters, imperative mood ("add" not "added")
- **body**: Explain *why*, not *what*
- **footer**: Reference issues: `Fixes #123`, `Closes #456`

### 4. Push & Create a Pull Request

```bash
git push origin feat/your-feature-name
```

Then open a PR on GitHub.

#### Pull Request Template

```markdown
## Description
What does this PR do?

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Checklist
- [ ] Code compiles with 0 warnings
- [ ] All tests pass (`cargo test`)
- [ ] Linter passes (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt`)
- [ ] New tests added
- [ ] Documentation updated
- [ ] No hardcoded secrets or credentials

## Testing
How did you test this?

## Related Issues
Fixes #123
```

### 5. Code Review

We'll review your PR and may ask for changes. Please:

- Respond to feedback promptly
- Don't be discouraged by revision requests (they make code better!)
- Ask questions if feedback is unclear

Once approved, a maintainer will merge your PR.

---

## Testing Requirements

### Relay Server (Rust)

```bash
cd nexus-relay

# All tests must pass
cargo test

# Linter must pass with 0 warnings
cargo clippy --all-targets -- -D warnings

# Formatter must be applied
cargo fmt --all

# Build release binary
cargo build --release

# Check for security vulnerabilities
cargo audit
```

### Web Client (TypeScript)

```bash
cd nexus-web

# Lint
npm run lint

# Format
npm run format

# Build
npm run build

# Type check
npm run type-check
```

### Integration Tests

```bash
# Start the full stack
docker compose -f docker-compose.prod.yml up -d

# Run load tests
cd nexus-relay/loadtest
k6 run k6-load-test.js
```

---

## Areas We Need Help With

### High Priority

- [ ] **Mobile Clients** — Kotlin (Android) and Swift (iOS) implementations
- [ ] **Security Audit** — Review crypto implementation
- [ ] **Performance Optimization** — Faster message decryption
- [ ] **Usability** — UX improvements for key verification

### Medium Priority

- [ ] **Documentation** — API docs, deployment guides
- [ ] **Translations** — Internationalization (i18n)
- [ ] **Monitoring** — More Grafana dashboards
- [ ] **CI/CD** — expand to more platforms (ARM, RISC-V)

### Low Priority

- [ ] **Examples** — Sample applications using the SDK
- [ ] **Benchmarks** — More comprehensive performance tests
- [ ] **Fuzzing** — Expand fuzz testing coverage

---

## Code Style Guide

### Rust

```rust
// Use full sentences for doc comments
/// This function does something important.
///
/// It achieves this by doing A, then B, then C.

// Imports in order: std, external, internal
use std::io;
use serde::{Deserialize, Serialize};
use crate::crypto;

// 100-character soft limit, hard limit at 120
let very_long_variable_name = SomeType::new()
    .with_option_one(true)
    .with_option_two(some_value)?;

// Unused variables: use _name to signal intent
let _unused = something();
```

### TypeScript

```typescript
// Use interfaces over types for object types
interface UserMessage {
  from: string;
  to: string;
  body: string;
}

// Export types publicly
export type MessageStatus = "sent" | "delivered" | "read";

// Use async/await, not promises
async function sendMessage(msg: UserMessage): Promise<void> {
  await api.post("/messages", msg);
}
```

---

## Documentation

### Updating Docs

- Edit markdown files directly
- Update table of contents if needed
- Test links: `markdown-link-check *.md`
- Follow the style of existing docs

### Generating API Docs

```bash
# Rust
cd nexus-relay && cargo doc --open

# TypeScript
cd nexus-web && npm run doc
```

---

## Security

### Some Dos and Don'ts

 **Do:**
- Report security issues privately (see [SECURITY.md](SECURITY.md))
- Add tests for security-critical code
- Use constant-time comparisons for secrets
- Zeroize sensitive memory
- Document threat assumptions

 **Don't:**
- Commit secrets (API keys, passwords, tokens)
- Use `unsafe` without justification
- Ignore compiler warnings
- Skip security tests
- Use deprecated cryptographic functions

---

## Review Process

### Automated Checks

Every PR runs:

- **Compilation check** — Code must compile
- **Linter** — `cargo clippy -- -D warnings`
- **Formatter** — `cargo fmt --check`
- **Tests** — All must pass
- **Code coverage** — Must not decrease
- **Security audit** — `cargo audit`
- **Dependency check** — No yanked dependencies

### Manual Review

A maintainer will review:

-  Code quality and style
-  Cryptographic correctness (for crypto changes)
-  Performance impact
-  Documentation completeness
-  Test coverage

---

## Releases

NEXUS follows [Semantic Versioning](https://semver.org/):

- **0.3.x** — Breaking changes (pre-1.0)
- **0.3.1** — Bug fixes, non-breaking features
- **0.3.2** — Critical patches

Major releases are cut roughly every 3 months.

To request a release, open an issue labeled `release`.

---

## Questions?

- **Contribution questions:** Open an issue or ask in discussions
- **Security concerns:** See [SECURITY.md](SECURITY.md)
- **Chat:** Join our Discord (link coming soon)

---

## License

By contributing to NEXUS, you agree that your contributions will be licensed under the same license as the project (AGPL-3.0 for relay, Apache-2.0 for crypto).

---

**Thank you for making NEXUS better!** 
