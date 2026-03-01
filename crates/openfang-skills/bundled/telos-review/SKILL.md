---
name: telos-review
description: "TELOS file freshness monitoring — detect stale goals, completed milestones, and outdated context to prompt timely user updates"
---
# TELOS Review & Freshness Monitoring

Periodically evaluate whether the user's TELOS files reflect their current reality. Stale TELOS data degrades every agent's output quality.

## Staleness Detection Rules

### Time-Based Triggers
| File | Review Interval | Rationale |
|------|----------------|-----------|
| GOALS.md | Every 2 weeks | Goals evolve frequently |
| PROJECTS.md | Every 2 weeks | Project status changes often |
| CHALLENGES.md | Every week | Challenges may resolve or escalate |
| STRATEGIES.md | Monthly | Strategy changes less frequently |
| MISSION.md | Quarterly | Mission should be stable |
| BELIEFS.md | Quarterly | Core values rarely change |
| MODELS.md | Quarterly | Mental models evolve slowly |
| IDEAS.md | Monthly | Ideas should be captured or discarded |
| LEARNED.md | Monthly | New lessons from recent activity |
| NARRATIVES.md | Quarterly | Life narrative evolves slowly |

### Content-Based Triggers
- **Completed goals**: If a goal was marked as due and the date has passed, prompt for status update
- **Milestone dates**: If PROJECTS.md mentions a milestone date that has passed, flag it
- **Stale challenges**: If a challenge references something that appears resolved based on recent activity, suggest removal
- **Empty files**: If a file exists but contains only the template placeholder, encourage the user to fill it in

### Activity-Based Triggers
- After a Hand completes a major deliverable, check if PROJECTS.md milestones should be updated
- After the user achieves a goal, check if GOALS.md needs the item moved to "Completed"
- After learning something significant (via LEARNED.md auto-append, if enabled), verify the lesson is captured

## Review Report Format

```markdown
## TELOS Freshness Review — [Date]

### Needs Attention
- ⚠️ GOALS.md: Goal "[goal]" was due [date], status unknown
- ⚠️ PROJECTS.md: Milestone "[milestone]" target date passed
- ⚠️ CHALLENGES.md: Last updated [N days] ago

### Suggestions
- 💡 Consider adding recent learnings from [Hand activity]
- 💡 IDEAS.md has [N] items older than 3 months — archive or pursue?

### Healthy
- ✅ MISSION.md: Current (updated [date])
- ✅ STRATEGIES.md: Current (updated [date])
```

## Implementation Notes
- This skill is advisory only — it never modifies TELOS files (per ADR-004)
- Reviews should be non-intrusive: one summary per review cycle, not per finding
- The user can trigger a manual review via `octarq telos status`
