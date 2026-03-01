---
name: telos-channel-routing
description: "TELOS-aware notification routing — select the optimal delivery channel based on message urgency, goal relevance, and user context"
---
# TELOS-Aware Channel Routing

Route notifications and deliveries to the right channel based on the user's TELOS context and message characteristics.

## Routing Decision Matrix

### Priority Classification

**P0 — Interrupt (use most direct channel)**:
- Threat to #1 active goal (e.g., competitor launch, critical bug)
- Time-sensitive opportunity aligned with GOALS.md
- Security or financial alert
- Channels: Push notification, SMS, Telegram (if configured)

**P1 — Same-day (use active communication channel)**:
- Progress update on active project (PROJECTS.md)
- New intelligence related to current challenge (CHALLENGES.md)
- Completed research deliverable
- Channels: Telegram, Slack, Email

**P2 — Digest (batch into periodic reports)**:
- Background monitoring updates
- Low-urgency intelligence findings
- Routine status updates
- Channels: Daily email digest, Dashboard

**P3 — Archive (store, don't notify)**:
- Tangential findings not tied to active goals
- Historical data for future reference
- Channels: Dashboard only, searchable via API

### Goal-Relevance Scoring

```
Score = (goal_priority × match_strength)

goal_priority:
  1.0 = Active goal #1
  0.7 = Active goal #2-3
  0.4 = Active but lower priority
  0.1 = Paused goal

match_strength:
  1.0 = Directly addresses the goal
  0.5 = Related to the goal's project
  0.2 = Tangentially related via challenge
  0.0 = No connection

Route: Score > 0.7 → P0/P1, Score > 0.3 → P2, else → P3
```

## Channel Selection Rules

1. Never send the same notification to multiple channels (reduces noise)
2. Respect time-of-day preferences if configured
3. Batch low-priority items into digests rather than individual messages
4. Include a one-line TELOS alignment note: "Related to: [goal/project/challenge]"
5. If the user hasn't configured channels, default to dashboard-only
