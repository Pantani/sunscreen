```markdown
# sunscreen Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill introduces the core development patterns and conventions used in the `sunscreen` Rust codebase. You'll learn about file naming, import/export styles, commit practices, and how to work with and run tests. This guide is ideal for contributors who want to quickly align with the project's established practices.

## Coding Conventions

### File Naming
- **Convention:** Use camelCase for file names.
- **Example:**  
  `sunscreenCore.rs`  
  `utilsParser.rs`

### Import Style
- **Convention:** Use relative imports within the codebase.
- **Example:**
  ```rust
  mod utils;
  use crate::utils::parseInput;
  ```

### Export Style
- **Convention:** Use named exports for modules and functions.
- **Example:**
  ```rust
  pub fn calculateSunProtection() { ... }
  ```

### Commit Patterns
- **Type:** Freeform, no strict prefixes required.
- **Average length:** ~38 characters per message.
- **Example:**  
  `fix bug in SPF calculation logic`

## Workflows

### Adding a New Module
**Trigger:** When you need to introduce new functionality.
**Command:** `/add-module`

1. Create a new file using camelCase (e.g., `newFeature.rs`).
2. Define your module with named exports.
3. Use relative imports to integrate with existing code.
4. Update main files to include your new module.

### Running Tests
**Trigger:** When you want to verify code correctness.
**Command:** `/run-tests`

1. Locate test files matching the `*.test.*` pattern.
2. Use the Rust test runner (e.g., `cargo test`) to execute tests.
3. Review output and address any failures.

### Refactoring Code
**Trigger:** When improving code structure or readability.
**Command:** `/refactor`

1. Rename files using camelCase if needed.
2. Update relative imports to match new file names.
3. Ensure all exports remain named and consistent.
4. Run tests to verify no regressions.

## Testing Patterns

- **Framework:** Not explicitly defined; likely uses Rust's built-in test framework.
- **File Pattern:** Test files are named with the `*.test.*` pattern (e.g., `mathUtils.test.rs`).
- **Example:**
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_sun_protection() {
          assert_eq!(calculateSunProtection(30), 0.97);
      }
  }
  ```
- **How to Run:**  
  Use `cargo test` or your preferred Rust test runner.

## Commands
| Command        | Purpose                                      |
|----------------|----------------------------------------------|
| /add-module    | Scaffold and integrate a new module          |
| /run-tests     | Run all tests in the codebase                |
| /refactor      | Refactor code and update imports/exports     |
```