---
description: 'Agent selection policy for the niri desktop environment project'
applyTo: '**/*'
---
# Agent Selection Policy

Use the right agent for every task. Each agent in `.github/agents/` has a clear specialty.

## Available Agents
| Agent | Use for |
|-------|---------|
| `Debug Mode Instructions` | Diagnosing and fixing runtime bugs, crashes, test failures |
| `Rust Reviewer` | Code review of `.rs` files for correctness, style, and project conventions |
| `SE: Security` | Security audits — OWASP Top 10, auth flows, input handling |

## Rules
- **Debug agent** → only for finding and fixing bugs. Not for new features or design.
- **Rust Reviewer** → read-only review. It produces a report, it does not edit code.
- **SE: Security** → use before merging any code that handles user input, IPC, or external data.
- If the task spans both UI and backend, start with a backend review before touching UI.
- When in doubt about scope, ask before choosing an agent.

## When to Use Each
- Build fails or test fails → Debug Mode Instructions
- PR ready for merge → Rust Reviewer + SE: Security
- New IPC or protocol code → SE: Security mandatory
- New GTK4 widget → no special agent needed (follow `ui-standards.md`)
