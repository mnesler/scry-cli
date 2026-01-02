# Development Workflow

## Testing Requirements

All new code must include tests. Before committing:

1. **Write tests** for new functionality
2. **Run `cargo test`** to verify all tests pass
3. **Enforce test coverage** - no commits without corresponding tests for new code

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test <module_name>
```

## Issue Management

### Closing Issues

After completing an issue, close it with the associated commit SHA:

```bash
gh issue close <issue-number> --comment "Resolved in commit $(git rev-parse HEAD)"
```

This links the implementation to the issue for future reference.

## Commit Format

Use conventional commits: `type(scope): description`

Types:
- `feat` - new feature
- `fix` - bug fix
- `refactor` - code restructuring
- `docs` - documentation only
- `test` - adding/updating tests
- `chore` - maintenance tasks
- `style` - formatting, no code change
