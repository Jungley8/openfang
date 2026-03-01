---
name: pai-research
description: "TELOS-aware research methodology — align all research with user goals, projects, and challenges from their personal context"
---
# TELOS-Aware Research

When the user has a TELOS context loaded, use it as the lens for all research.

## Goal-Oriented Research

### Alignment Scoring
For every research finding, evaluate its relevance:

```
HIGH ALIGNMENT: Directly advances an active goal in GOALS.md
  → Prioritize in report, recommend immediate action

MEDIUM ALIGNMENT: Relates to a current project in PROJECTS.md
  → Include in report with actionable framing

LOW ALIGNMENT: Interesting but tangential to user's mission
  → Mention briefly, flag as "peripheral"

OFF-TARGET: Doesn't connect to any TELOS context
  → Omit unless explicitly requested
```

### Research Targeting from TELOS

1. **From GOALS.md**: Each active goal implies research questions
   - "Complete MVP by Q2" → research comparable MVP timelines, common blockers
   - "Raise Pre-A $500K" → research investor expectations, comparable deals

2. **From PROJECTS.md**: Each project implies competitive landscape
   - Extract competitor names and product positioning
   - Monitor technology dependencies and alternatives

3. **From CHALLENGES.md**: Each challenge implies solution-seeking
   - Research who else has faced this challenge
   - Find case studies of successful resolution
   - Identify experts or resources

## Report Structure for TELOS Users

```markdown
# Research: [Topic]

## TELOS Alignment
- Goal: [Which goal this advances]
- Relevance: [HIGH/MEDIUM/LOW]

## Key Findings
[Findings ordered by goal relevance, not chronology]

## Recommended Actions
[Specific next steps tied to GOALS.md or PROJECTS.md]

## Impact on Challenges
[How findings affect CHALLENGES.md items]
```

## Anti-Patterns
- Do not produce generic research that ignores the user's context
- Do not repeat information the user already knows (check LEARNED.md)
- Do not recommend strategies that conflict with BELIEFS.md
- Do not suggest approaches the user has already tried and failed (check LEARNED.md)
