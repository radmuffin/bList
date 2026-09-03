# Agent Sober Findings

## Removed AI-style comments

- Removed conversational, prompt-referencing comments from:
  - `src/main.rs`

## Notes

- The removed comments did not affect runtime behavior.
- `/metrics` remains exposed at the top-level route on `app_router`.
