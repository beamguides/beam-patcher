# Contributing to Beam Patcher

First off, thank you for considering contributing to Beam Patcher! 🎉

This document provides guidelines for contributing to the project. Following these guidelines helps maintain code quality and makes the contribution process smooth for everyone.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Setup](#development-setup)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Community](#community)

## Code of Conduct

This project follows a simple code of conduct: **Be respectful and constructive**. We're all here to make RO patching better for the community.

## How Can I Contribute?

### 🐛 Reporting Bugs

Before creating a bug report, please check existing issues to avoid duplicates.

**When reporting a bug, include:**
- Clear and descriptive title
- Steps to reproduce the issue
- Expected vs actual behavior
- Your environment (OS, Rust version, etc.)
- Error messages or logs (if any)
- Screenshots (if applicable)

### 💡 Suggesting Features

Feature suggestions are welcome! Please:
- Use GitHub Discussions for feature ideas
- Explain the use case and benefits
- Consider backwards compatibility
- Be open to feedback and discussion

### 🎨 Creating Themes

Share your custom themes with the community:

1. Create a theme in `themes/` folder
2. Include a `README.md` with preview screenshots
3. Provide `config.yml` and `assets/` folder
4. Submit a PR with the theme

### 📝 Improving Documentation

Documentation improvements are always appreciated:
- Fix typos or unclear explanations
- Add examples or use cases
- Translate documentation (future)
- Create tutorials or guides

### 🔧 Code Contributions

#### What to Work On

**Good first issues:**
- Bug fixes with clear reproduction steps
- Documentation improvements
- UI/UX enhancements
- Test coverage improvements

**Advanced contributions:**
- New file format support
- Performance optimizations
- New features (discuss first!)
- Refactoring (discuss first!)

## Development Setup

### Prerequisites

- Rust 1.75 or higher
- Node.js 18+ (for Tauri)
- Git

### Setup Steps

```bash
# Clone the repository
git clone https://github.com/beamguide/beam-patcher.git
cd beam-patcher

# Build the project
cargo build

# Run tests
cargo test

# Run the patcher in development mode
cargo run --bin beam-patcher

# Check code formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

### Project Structure

```
beam-patcher/
├── beam-core/          # Core patching logic
├── beam-formats/       # File format implementations
├── beam-patcher/       # Main executable
├── beam-ui/            # Tauri GUI
├── examples/           # Example configurations
├── themes/             # Community themes
└── docs/               # Documentation
```

## Pull Request Process

### Before Submitting

1. **Discuss major changes** in GitHub Discussions or Issues first
2. **Create a feature branch** from `master`
3. **Write clear commit messages** (see below)
4. **Add tests** for new functionality
5. **Update documentation** if needed
6. **Run tests and linter** before submitting

### Commit Messages

Follow conventional commit format:

```
type(scope): short description

Longer description if needed

Fixes #issue-number
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style/formatting (no logic change)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(ui): add dark mode theme support

fix(grf): handle corrupted file tables gracefully
Fixes #42

docs(readme): add theme customization guide

refactor(downloader): improve error handling
```

### PR Template

When creating a PR, include:

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Theme contribution
- [ ] Refactoring

## Testing
- [ ] Tests pass locally
- [ ] Added new tests (if applicable)
- [ ] Manual testing completed

## Screenshots (if applicable)
[Add screenshots here]

## Related Issues
Fixes #issue-number
```

### Review Process

1. Maintainer will review your PR within 1-2 weeks
2. Address any feedback or requested changes
3. Once approved, PR will be merged
4. Your contribution will be acknowledged in release notes

## Coding Standards

### Rust Code Style

- Follow official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Write idiomatic Rust code

### Code Quality

**DO:**
- ✅ Write clear, self-documenting code
- ✅ Add comments for complex logic
- ✅ Handle errors explicitly (avoid `unwrap()` in production code)
- ✅ Write unit tests for new functionality
- ✅ Use meaningful variable and function names
- ✅ Keep functions small and focused

**DON'T:**
- ❌ Use `unsafe` without justification
- ❌ Panic in library code
- ❌ Commit commented-out code
- ❌ Add unnecessary dependencies
- ❌ Ignore compiler warnings

### Performance

- Profile before optimizing
- Document performance-critical sections
- Consider memory usage for large files
- Use async/await for I/O operations

### Security

- Never commit secrets or API keys
- Validate all user input
- Use secure random number generation
- Follow OWASP guidelines for web content

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run specific package tests
cargo test -p beam-formats
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = create_test_data();
        
        // Act
        let result = function_to_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

## Documentation

### Code Documentation

- Document public APIs with doc comments
- Include examples in documentation
- Use `cargo doc --open` to preview

```rust
/// Downloads a patch file from the specified URL.
///
/// # Arguments
///
/// * `url` - The URL to download from
/// * `output_path` - Where to save the downloaded file
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if download fails.
///
/// # Examples
///
/// ```
/// download_patch("https://example.com/patch.beam", "patch.beam")?;
/// ```
pub fn download_patch(url: &str, output_path: &Path) -> Result<()> {
    // Implementation
}
```

### User Documentation

Update relevant docs in `docs/` folder:
- `CONFIGURATION.md` - Configuration options
- `THEMES.md` - Theme creation guide
- `BUILDING.md` - Build instructions

## Community

### Getting Help

- **GitHub Discussions**: Ask questions, share ideas
- **GitHub Issues**: Report bugs, request features
- **Discord**: beamguide#9797

### Staying Updated

- Watch the repository for updates
- Read release notes for new versions
- Follow project announcements

### Recognition

Contributors will be recognized in:
- `CHANGELOG.md` for each release
- GitHub contributors page
- Special mentions for significant contributions

## License

By contributing, you agree that your contributions will be dual-licensed under MIT and Apache-2.0, the same as the project.

---

**Thank you for contributing to Beam Patcher!** 🎉

Your contributions help make RO patching better for the entire private server community.

**Questions?** Feel free to ask in GitHub Discussions or contact [@beamguide](https://github.com/beamguide).
