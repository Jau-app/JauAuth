# Contributing to JauAuth

Thank you for your interest in contributing to JauAuth! We welcome contributions from the community and are grateful for any help you can provide.

## 🤝 Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct:
- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive criticism
- Respect different viewpoints and experiences

## 🚀 Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/Jau-Auth.git
   cd Jau-Auth
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/Jau-app/Jau-Auth.git
   ```
4. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## 🛠️ Development Setup

### Prerequisites
- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- Node.js 18+ and npm
- Git

### Building the Project
```bash
# Install TypeScript dependencies
cd mcp-server && npm install && cd ..

# Build everything
cargo build
cd mcp-server && npm run build && cd ..

# Run tests
cargo test
```

### Running Locally
```bash
# Start the development server
./run-combined-server.sh

# In another terminal, run the TypeScript watcher
cd mcp-server && npm run dev
```

## 📝 Making Changes

### Before You Start
- Check existing [issues](https://github.com/Jau-app/Jau-Auth/issues) to avoid duplicates
- For major changes, open an issue first to discuss
- Make sure your code follows the existing style

### Commit Guidelines
We follow conventional commits:
- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `style:` Code style changes (formatting, etc.)
- `refactor:` Code refactoring
- `test:` Test additions or changes
- `chore:` Maintenance tasks

Example:
```bash
git commit -m "feat: add Docker sandbox support for Windows"
```

### Code Style

#### Rust
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix any warnings
- Add tests for new functionality

#### TypeScript
- Run `npm run lint` in the `mcp-server` directory
- Follow the existing code style
- Add JSDoc comments for public functions

### Testing
- Write tests for new features
- Ensure all tests pass: `cargo test`
- Test with real MCP servers when possible

## 🔄 Pull Request Process

1. **Update your fork**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Push your changes**:
   ```bash
   git push origin feature/your-feature-name
   ```

3. **Create a Pull Request**:
   - Go to your fork on GitHub
   - Click "New Pull Request"
   - Provide a clear title and description
   - Reference any related issues

4. **PR Requirements**:
   - ✅ All tests pass
   - ✅ Code is formatted (`cargo fmt` and `npm run lint`)
   - ✅ Documentation is updated if needed
   - ✅ Commit messages follow guidelines
   - ✅ PR description explains the changes

## 🐛 Reporting Issues

### Bug Reports
Please include:
- Clear description of the bug
- Steps to reproduce
- Expected behavior
- Actual behavior
- System information (OS, Rust version, Node version)
- Relevant logs or error messages

### Feature Requests
Please include:
- Clear description of the feature
- Use case and motivation
- Possible implementation approach (optional)

## 📚 Documentation

- Update README.md for user-facing changes
- Update CLAUDE.md for development guidance
- Add inline comments for complex code
- Update API documentation if endpoints change

## 🏷️ Versioning

We use [Semantic Versioning](https://semver.org/):
- MAJOR version for incompatible API changes
- MINOR version for backwards-compatible functionality
- PATCH version for backwards-compatible bug fixes

## ⚡ Quick Contribution Ideas

Looking for something to work on? Check out:
- Issues labeled `good first issue`
- Issues labeled `help wanted`
- TODO comments in the code
- Missing tests
- Documentation improvements

## 🙏 Recognition

Contributors will be:
- Added to the README contributors section
- Mentioned in release notes
- Given credit in commit messages

## 📞 Getting Help

- Open an issue for bugs or features
- Join our [Discord](https://discord.gg/jau-app) for discussions
- Tag maintainers in PR comments if you need review

## 🚀 Release Process

Maintainers will:
1. Review and merge PRs
2. Update version numbers
3. Update CHANGELOG.md
4. Create GitHub releases
5. Publish to crates.io (Rust) and npm (TypeScript)

---

Thank you for contributing to JauAuth! Your efforts help make MCP deployments more secure and manageable for everyone. 🎉