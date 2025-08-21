# Contributing to Saros SDK Documentation

Thank you for your interest in contributing to the Saros SDK documentation! This guide will help you get started.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Documentation Structure](#documentation-structure)
- [Writing Guidelines](#writing-guidelines)
- [Code Examples](#code-examples)
- [Pull Request Process](#pull-request-process)

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming environment for all contributors.

## How to Contribute

### Reporting Issues

- Check existing issues before creating a new one
- Use clear, descriptive titles
- Include relevant code examples or error messages
- Specify SDK version and environment details

### Suggesting Improvements

- Open an issue with the "enhancement" label
- Clearly describe the proposed improvement
- Explain the use case and benefits

### Contributing Documentation

1. Fork the repository
2. Create a feature branch (`git checkout -b improve-swap-docs`)
3. Make your changes
4. Test code examples
5. Submit a pull request

## Documentation Structure

```
saros-sdk-docs/
├── api-reference/      # API documentation for each SDK
├── architecture/       # System architecture and diagrams
├── code-examples/      # Complete, runnable examples
├── core-concepts/      # Fundamental concepts
├── getting-started/    # Quick start guides
├── guides/            # In-depth guides and tutorials
└── tutorials/         # Step-by-step tutorials
```

### Adding New Documentation

- **API Reference**: Technical specifications and method signatures
- **Guides**: Conceptual explanations and best practices
- **Tutorials**: Step-by-step instructions for specific tasks
- **Code Examples**: Complete, runnable applications

## Writing Guidelines

### Style Guide

- Use clear, concise language
- Write in present tense
- Use active voice
- Include code examples for complex concepts
- Add diagrams where helpful

### Markdown Formatting

```markdown
# H1 - Page Title
## H2 - Major Sections
### H3 - Subsections

**Bold** for emphasis
`code` for inline code
- Bullet points for lists
1. Numbered lists for steps

\```typescript
// Code blocks with syntax highlighting
\```
```

### Code Example Requirements

1. **Complete**: Examples should be runnable without modification
2. **Commented**: Include explanatory comments
3. **Error Handling**: Demonstrate proper error handling
4. **TypeScript**: Use TypeScript with proper types
5. **Best Practices**: Follow SDK best practices

### Documentation Checklist

- [ ] Clear title and description
- [ ] Table of contents for long documents
- [ ] Code examples tested and working
- [ ] Links to related documentation
- [ ] Proper markdown formatting
- [ ] No sensitive information (keys, passwords)

## Code Examples

### Structure

```typescript
/**
 * Example: [Clear description of what this example demonstrates]
 * 
 * This example shows how to [specific functionality]
 * 
 * Prerequisites:
 * - Solana wallet with SOL
 * - Node.js >= 16
 * - @saros-finance/sdk installed
 */

import { /* required imports */ } from '@saros-finance/sdk';

// Configuration
const config = {
  // ...
};

// Main function with error handling
async function main() {
  try {
    // Implementation
  } catch (error) {
    console.error('Error:', error);
  }
}

// Run if executed directly
if (require.main === module) {
  main();
}
```

### Testing Examples

Before submitting:

1. Install dependencies: `npm install`
2. Set up environment variables
3. Run the example: `npm run dev`
4. Verify expected output

## Pull Request Process

### Before Submitting

1. **Test all code examples**
2. **Run linter**: `npm run lint`
3. **Check formatting**: `npm run format`
4. **Update relevant documentation**
5. **Add/update tests if applicable**

### PR Title Format

Use conventional commit format:
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation only
- `refactor:` Code restructuring
- `test:` Test additions/changes
- `chore:` Maintenance tasks

Examples:
- `docs: Add DLMM position management guide`
- `feat: Add multi-hop routing example`
- `fix: Correct slippage calculation in swap example`

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Documentation update
- [ ] New code example
- [ ] Bug fix
- [ ] New feature

## Testing
- [ ] Code examples tested
- [ ] Links verified
- [ ] Markdown formatted correctly

## Checklist
- [ ] My code follows the style guidelines
- [ ] I have performed a self-review
- [ ] I have commented my code where necessary
- [ ] My changes generate no new warnings
```

## Questions?

- Join our [Discord](https://discord.gg/saros)
- Check existing [issues](https://github.com/saros-finance/sdk-docs/issues)
- Contact the team at docs@saros.finance

## License

By contributing, you agree that your contributions will be licensed under the MIT License.