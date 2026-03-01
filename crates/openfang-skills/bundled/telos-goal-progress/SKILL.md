---
name: telos-goal-progress
description: "Goal alignment progress reporting — aggregate Hand activity and deliverables into TELOS goal completion tracking"
---
# TELOS Goal Progress Tracking

Aggregate all Hand activity, deliverables, and intelligence into a goal-centric progress view. Answer the question: "How close am I to achieving my goals?"

## Activity-to-Goal Mapping

### Automatic Association
For each Hand task completion, map the output to TELOS goals:

```
Task: [Hand] completed [deliverable]
  → Match against GOALS.md active goals using keyword overlap
  → Match against PROJECTS.md milestones
  → Match against CHALLENGES.md items
  → Assign alignment_score (0.0 to 1.0)
```

### Alignment Score Heuristics
- **1.0**: Deliverable directly completes a goal milestone
- **0.8**: Deliverable provides critical input for a goal (e.g., research for a decision)
- **0.5**: Deliverable is related to a goal's project
- **0.2**: Deliverable is tangentially related
- **0.0**: No connection to any active goal (flag as "unaligned work")

## Goal Progress Report

```markdown
# Goal Progress Report — [Period]

## Active Goals

### 🎯 [Goal 1 from GOALS.md]
**Progress**: [Estimated %] | **Due**: [Date]
**Recent contributions**:
- [Hand] delivered [output] — alignment: 0.8
- [Hand] delivered [output] — alignment: 0.5
**Blockers**: [From CHALLENGES.md if applicable]
**Recommendation**: [Next action to advance this goal]

### 🎯 [Goal 2 from GOALS.md]
...

## Alignment Summary
| Metric | Value |
|--------|-------|
| Total Hand tasks this period | N |
| Goal-aligned tasks (score > 0.5) | N (X%) |
| Unaligned tasks | N (X%) |
| Average alignment score | 0.XX |
| Most active goal | [Goal name] |
| Least served goal | [Goal name] |

## Unaligned Activity
[List tasks with alignment_score < 0.2 — these may indicate scope creep or untracked goals]

## Recommendations
- [Goal with lowest activity] needs more attention
- Consider updating GOALS.md if priorities have shifted
- [Challenge] appears to be blocking [Goal] — escalate?
```

## Scoring Period
- Default: 30 days (matches `octarq telos report --days 30`)
- The report uses TELOS snapshot history to detect goal changes over time
- If a goal was added mid-period, only count activity after the addition

## Usage
This skill's output powers:
- `octarq telos report` CLI command
- `/api/telos/report` API endpoint
- Dashboard goal progress widgets
