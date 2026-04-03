name: Release
description: "🚀 Release a new version of NEXUS"
title: "[RELEASE] v0.x.x"
labels: ["release"]
assignees: []

---

# Release Checklist for NEXUS v0.x.x

## Pre-Release

- [ ] Verify all tests pass: `cd nexus-relay && cargo test`
- [ ] Verify 0 warnings: `cargo clippy --all-targets -- -D warnings`
- [ ] Verify formatting: `cargo fmt --check`
- [ ] Run security audit: `cargo audit`
- [ ] Update CHANGELOG.md with all changes
- [ ] Update version in all `Cargo.toml` files
- [ ] Update version in `nexus-web/package.json`
- [ ] Update version in documentation

## Documentation

- [ ] Update README.md if needed
- [ ] Review SECURITY.md for new vulnerabilities
- [ ] Add migration guide if breaking changes
- [ ] Tag release with security notes if applicable

## Testing

- [ ] Test full Docker stack: `docker compose -f docker-compose.prod.yml up`
- [ ] Run load test: `cd nexus-relay/loadtest && k6 run k6-load-test.js`
- [ ] Test on Kubernetes (if available)
- [ ] Verify binary size (target: <10MB): `ls -lh releases/`

## Release

- [ ] Create git tag: `git tag -a v0.x.x -m "Release v0.x.x - Description"`
- [ ] Push tag: `git push origin v0.x.x`
- [ ] Create GitHub Release with CHANGELOG entry
- [ ] Upload binaries to Release page

## Post-Release

- [ ] Announce on communication channels
- [ ] Update website/docs
- [ ] Monitor for issues
- [ ] Backport critical fixes to previous version (if needed)

---

## Release Notes Template

```markdown
## v0.x.x — YYYY-MM-DD

### Security

### Features

### Improvements

### Bug Fixes

### Breaking Changes

### Dependencies

### Contributors

Thanks to @user1, @user2 for contributions to this release!
```

---

## Semantic Versioning

- **0.x.0** — Major features, breaking changes
- **0.x.y** — Bug fixes, non-breaking features
- **0.x.z** — Critical security patches

---

**Release Manager**: @username
**Date**: YYYY-MM-DD
