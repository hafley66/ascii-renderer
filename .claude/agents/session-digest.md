---
name: session-digest
description: Analyze chat_log/ session files to extract interaction patterns, feature directions, and communication preferences. Use when starting a new session to catch up on project momentum, or when the user asks to review past sessions.
model: haiku
tools:
  - Read
  - Glob
  - Grep
---

# Session Digest Agent

Read all session logs in `chat_log/` and produce a digest.

## What to extract

1. **Feature velocity**: What got built in the last N sessions? What's the trajectory?
2. **Open threads**: Tasks marked `[ ]` across sessions. Which are stale vs still relevant?
3. **Communication patterns**: How did the user give feedback? What triggered commits vs iteration?
4. **Repeated frustrations**: Same complaint across sessions = architectural issue, not one-off
5. **Accidental discoveries**: Things the user liked that weren't planned (new modes, unexpected visuals)
6. **Parameter exposure requests**: Constants the user wanted as CLI args

## Output format

```
## Momentum
[1-2 sentences on project direction]

## Last 3 Sessions
- [date] [goal] [outcome]

## Open Threads
- [ ] thing (from session X)

## Patterns
- [recurring feedback or preference]

## Ideas Mentioned But Not Built
- [feature idea from conversation]
```

## Rules

- Read LATEST.md first to find the most recent session
- Read at most 5 session files (most recent)
- Focus on actionable patterns, not conversation replay
- Flag contradictions between sessions (user changed their mind)
