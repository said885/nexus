# Welcome to NEXUS Contributors

Thank you for your interest in contributing to NEXUS! This guide will help you get started.

## Quick Start for Contributors

1. **Fork** the repository
2. **Clone** your fork: `git clone https://github.com/YOUR_USERNAME/nexus.git`
3. **Create a branch**: `git checkout -b feature/your-feature-name`
4. **Make changes** and test locally
5. **Ensure quality**:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   ```
6. **Push** to your fork
7. **Create a Pull Request** with a detailed description

## Code Standards

### Rust Code
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for common mistakes
- Write tests for new features
- Add doc comments to public APIs
- No unsafe code without justification

### Commit Messages
```
type(scope): description

Optional detailed explanation

Fixes #123
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`

## Areas We Need Help With

### High Priority
- [ ] Security audits and penetration testing
- [ ] Performance optimization (target: sub-1ms encryption)
- [ ] Mobile client completion (Android/iOS)
- [ ] Documentation improvements

### Medium Priority
- [ ] Additional test coverage
- [ ] Benchmark suite expansion
- [ ] Integration with key management systems
- [ ] Hardware security module support

### Low Priority
- [ ] Code examples and tutorials
- [ ] Community translations

## Community Expectations

- Be respectful and constructive
- Follow our Code of Conduct
- No discussions of illegal activities
- Assume good intent

## Security Responsibly

Found a security issue? **Do NOT open an issue.**
Email: frensh5@proton.me

## Questions?

- **GitHub Issues**: For bugs and features
- **GitHub Discussions**: For questions and ideas
- **Email**: frensh5@proton.me

---

**Thank you for making NEXUS more secure for everyone!**
