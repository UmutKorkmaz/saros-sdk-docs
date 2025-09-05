# Contributing to Saros SDK Documentation

Thank you for your interest in contributing to the Saros SDK documentation! This guide will help you understand our standards and workflow.

## 🚀 Getting Started

### Prerequisites
- Node.js 16+ and npm/yarn
- Rust 1.70+ (for Rust examples)
- Git and familiarity with GitHub workflow
- Basic understanding of Markdown and documentation best practices

### Repository Structure
```
saros-sdk-docs/
├── docs/                          # Main documentation
│   ├── getting-started/           # Setup guides  
│   ├── api-reference/            # SDK API docs
│   ├── tutorials/                # Step-by-step tutorials
│   ├── guides/                   # Advanced guides
│   ├── core-concepts/            # Conceptual explanations
│   └── architecture/             # System architecture
├── code-examples/                # Working code examples
│   ├── typescript/               # TypeScript examples
│   └── rust/                     # Rust examples
├── public/                       # HTML documentation site
└── README.md                     # Project overview
```

## 📝 Documentation Standards

### Writing Style
- **Clarity First**: Write for developers at all skill levels
- **Concise**: Avoid unnecessary jargon or verbose explanations
- **Actionable**: Include practical examples and working code
- **Visual**: Use diagrams, code snippets, and structured layouts

### Markdown Conventions
```markdown
# H1 - Page Title (only one per document)
## H2 - Major Sections  
### H3 - Subsections
#### H4 - Minor subsections (avoid going deeper)

Use **bold** for emphasis, `code` for inline code
Use > blockquotes for important notes
Use ✅ ❌ for do/don't examples
Use 🔥 🚀 💡 emojis sparingly for visual appeal
```

### Code Examples Standards
- **Always Working**: All code examples must compile and run
- **Self-Contained**: Include all necessary imports and setup
- **Well-Commented**: Explain complex logic inline
- **Error Handling**: Show proper error handling patterns
- **TypeScript**: Prefer TypeScript over JavaScript for type safety

## 🎯 Contribution Workflow

### 1. Setting Up Development Environment
```bash
# Clone repository
git clone https://github.com/saros-finance/saros-sdk-docs
cd saros-sdk-docs

# Install dependencies
npm install

# For Rust examples
cd code-examples/rust
cargo build --release

# Start local development server (if applicable)
npm run dev
```

### 2. Making Changes

#### Documentation Updates
- Check existing [issues](https://github.com/saros-finance/sdk-docs/issues)
- Create new branch: `git checkout -b docs/your-feature-name`
- Make changes following our [style guide](#documentation-standards)
- Test all code examples work correctly
- Update navigation/links if adding new pages

#### Code Examples
- Follow existing example structure
- Include comprehensive README.md
- Add proper error handling and logging
- Test on both devnet and mainnet-beta (where applicable)
- Include CLI usage examples

### 3. Commit Message Standards

Follow our established commit patterns:

**Format**: `[Type] [Component/Area]: [Description]`

**Types**:
- `Add` - New features, documentation, or examples
- `Update` - Modifications to existing content
- `Fix` - Bug fixes or corrections
- `Organize` - Restructuring or reorganization
- `Remove` - Deletions or cleanup

**Examples**:
```bash
# Good commit messages (following project pattern)
Add getting-started: wallet setup comprehensive guide
Update typescript-examples: enhance error handling patterns  
Fix api-reference: correct DLMM SDK method signatures
Organize documentation: consolidate guides structure
Add rust-examples: implement multi-hop routing with A* algorithm

# Avoid
Update docs
Fix typo
Add stuff
```

**Component Areas**:
- `getting-started` - Setup and basic guides
- `api-reference` - SDK documentation  
- `tutorials` - Step-by-step guides
- `examples` - Code examples
- `guides` - Advanced guides
- `core-concepts` - Conceptual docs
- `rust-examples` - Rust-specific examples
- `typescript-examples` - TypeScript-specific examples
- `documentation` - Meta-documentation changes

### 4. Pull Request Process

```bash
# Create feature branch
git checkout -b docs/wallet-integration-guide

# Make changes and test
git add .
git commit -m "Add getting-started: comprehensive wallet integration guide"

# Push branch
git push origin docs/wallet-integration-guide

# Create PR on GitHub with:
# - Clear title following commit message format
# - Description of what was added/changed
# - Links to any issues being resolved
# - Screenshots for UI/visual changes
```

### 5. PR Review Checklist

Before submitting:
- [ ] All code examples compile and run successfully
- [ ] Documentation follows markdown conventions
- [ ] Internal links work correctly
- [ ] External links are valid and appropriate
- [ ] Commit messages follow project standards
- [ ] Changes are focused and atomic
- [ ] README files updated if new examples added

## 📋 Content Guidelines

### API Documentation
- **Complete Coverage**: Document all public methods and interfaces
- **Parameter Details**: Include types, default values, and constraints
- **Return Values**: Document return types and possible values
- **Error Conditions**: List possible errors and handling strategies
- **Usage Examples**: Provide practical code examples for each method

### Tutorial Structure
```markdown
# Tutorial Title

Brief description and learning objectives.

## Prerequisites
- Required knowledge
- Setup requirements

## Step 1: Setup
Code examples with explanations

## Step 2: Implementation  
More code with detailed explanations

## Step 3: Testing
How to verify the implementation

## Troubleshooting
Common issues and solutions

## Next Steps
Links to related content
```

### Example Standards
Each code example must include:
- **README.md** with setup and usage instructions
- **Working code** that compiles without modification
- **Package.json/Cargo.toml** with all dependencies
- **Error handling** and user-friendly messages
- **Comments** explaining complex logic
- **CLI interface** where appropriate

## 🔧 Technical Requirements

### Dependencies Management
- Pin exact versions for reproducibility
- Document any system requirements
- Use lockfiles (package-lock.json, Cargo.lock)
- Test with minimum required versions

### Testing Standards
```bash
# TypeScript examples
npm test                 # Unit tests
npm run lint            # Code style checks
npm run type-check      # TypeScript compilation

# Rust examples  
cargo test              # Unit tests
cargo clippy            # Lint checks
cargo check             # Compilation check
```

### Performance Considerations
- Optimize for fast loading and reading
- Keep examples focused and minimal
- Use efficient algorithms in code examples
- Consider mobile/responsive design for HTML docs

## 🎨 Visual Standards

### Diagrams and Charts
- Use Mermaid for technical diagrams
- Consistent color schemes across diagrams
- Clear labels and readable text
- Architecture diagrams for complex systems

### Screenshots
- High resolution (2x for retina displays)
- Consistent browser/environment
- Highlight relevant UI elements
- Include captions explaining context

### Code Formatting
- Use syntax highlighting for all code blocks
- Consistent indentation (2 spaces for JS/TS, 4 for Rust)
- Line length under 80 characters where possible
- Meaningful variable names

## 🐛 Issue Reporting

When reporting documentation issues:

```markdown
**Issue Type**: [Bug/Enhancement/Question]
**Area**: [getting-started/api-reference/examples/etc.]
**Description**: Clear description of the issue
**Expected**: What should happen
**Actual**: What currently happens
**Steps to Reproduce**: If applicable
**Environment**: OS, Node version, etc.
```

## 🏷️ Labeling Conventions

Use these labels for issues and PRs:
- `documentation` - Doc updates/fixes
- `examples` - Code example changes
- `enhancement` - New features/improvements  
- `bug` - Fixes for existing content
- `typescript` - TypeScript-specific
- `rust` - Rust-specific
- `good-first-issue` - Beginner-friendly
- `help-wanted` - Needs community input

## 🎉 Recognition

Contributors will be:
- Listed in project README
- Mentioned in release notes
- Eligible for community recognition
- Invited to contributor discussions

## 📚 Resources

- [Markdown Guide](https://www.markdownguide.org/)
- [Mermaid Documentation](https://mermaid-js.github.io/mermaid/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)
- [Rust Book](https://doc.rust-lang.org/book/)

## 🆘 Getting Help

- 💬 **Discord**: [Saros Dev Community](https://discord.gg/saros)
- 🐛 **Issues**: [GitHub Issues](https://github.com/saros-finance/sdk-docs/issues)
- 📧 **Email**: docs@saros.finance

---

## Commit Message Quick Reference

```bash
# Adding new content
Add getting-started: wallet setup comprehensive guide
Add typescript-examples: auto-compound yield optimization  
Add api-reference: Rust SDK complete method documentation

# Updating existing content
Update tutorials: enhance swap example with error handling
Update rust-examples: optimize memory usage in DLMM calculator
Update documentation: improve navigation structure

# Fixing issues
Fix api-reference: correct TypeScript interface definitions
Fix examples: resolve compilation errors in range orders
Fix links: update broken references in installation guide

# Organizing/restructuring
Organize documentation: consolidate guides structure  
Organize examples: standardize README format across projects
Organize repository: improve directory structure and navigation
```

Thank you for contributing to Saros SDK documentation! 🚀