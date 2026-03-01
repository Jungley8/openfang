---
name: pai-osint
description: "TELOS-driven OSINT intelligence collection — auto-derive monitoring targets and collection priorities from user's personal goal system"
---
# TELOS-Driven Intelligence Collection

Derive all monitoring targets and collection priorities directly from the user's TELOS files. The user should never need to manually specify what to monitor.

## Auto-Derived Monitoring Targets

### From PROJECTS.md
For each project, automatically monitor:
- Direct competitors (extract from project descriptions and competitive references)
- Technology dependencies (libraries, platforms, APIs the project uses)
- Market segment news (industry keywords from project descriptions)
- Key personnel at competitor organizations

### From CHALLENGES.md
For each challenge, automatically monitor:
- Solution providers and emerging tools
- Case studies of similar challenges being solved
- Expert commentary and conference talks
- Regulatory or market changes that affect the challenge

### From GOALS.md
For each active goal, automatically monitor:
- Success path signals (indicators that the goal is achievable)
- Risk signals (threats to goal completion)
- Benchmark data (how others are performing on similar goals)
- Timeline-relevant events (deadlines, market windows)

## Collection Priority Matrix

```
CRITICAL (real-time alerts):
  Items that directly threaten or advance the #1 active goal
  → Notify immediately via highest-priority channel

HIGH (daily digest):
  Items related to active projects or current challenges
  → Include in daily intelligence brief

MEDIUM (weekly report):
  Items related to secondary goals or future projects
  → Include in weekly summary

LOW (archive):
  Items tangentially related to user's mission
  → Store for future reference, don't actively report
```

## TELOS Change Detection

When the user updates their TELOS files:
1. Compare new goals/projects/challenges against the previous set
2. Identify new monitoring targets that weren't tracked before
3. Identify targets that are no longer relevant
4. Report: "Based on your updated goals, I've added monitoring for X and removed Y"

## Intelligence Brief Format

```markdown
# Intelligence Brief — [Date]
**TELOS Alignment**: [Which goals/projects this brief serves]

## Priority Alerts
[Items requiring immediate attention, linked to specific goals]

## Goal Progress Signals
[New data points relevant to active goal tracking]

## Competitive Moves
[Changes detected in competitor landscape from PROJECTS.md]

## Challenge Updates
[New information relevant to CHALLENGES.md items]
```
