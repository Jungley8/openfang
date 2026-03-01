---
name: pai-prompting
description: "TELOS-aware meta-prompting — context injection patterns, adaptive prompt construction, and multi-model strategies using the user's personal goal system"
---
# TELOS-Aware Meta-Prompting

Techniques for constructing more effective prompts by leveraging the user's TELOS context.

## Context Injection Principles

### What to Inject and When

| TELOS File | Inject When | Purpose |
|------------|------------|---------|
| MISSION.md | Always (for any substantive task) | Prevents off-mission drift |
| GOALS.md | Task relates to planning, prioritization, or resource allocation | Provides decision criteria |
| PROJECTS.md | Task involves a specific project | Grounds the task in project reality |
| CHALLENGES.md | Task is problem-solving or research | Focuses on actual blockers |
| BELIEFS.md | Task involves decisions with ethical dimensions | Prevents value misalignment |
| MODELS.md | Task requires analytical reasoning | Uses user's preferred frameworks |
| STRATEGIES.md | Task involves communication or content creation | Maintains consistent voice |
| NARRATIVES.md | Task involves storytelling or personal branding | Provides authentic material |
| LEARNED.md | Task is similar to something done before | Avoids repeating mistakes |
| IDEAS.md | Task is creative or exploratory | Seeds with user's own ideas |

### Prompt Adaptation by Task Type

**Strategic decisions**: Inject MISSION + GOALS + MODELS + BELIEFS
```
Given my mission of [MISSION], and my current goals [GOALS], using
the [MODEL] framework, evaluate [options]. Note any conflicts with
my core beliefs about [BELIEFS].
```

**Content creation**: Inject MISSION + STRATEGIES + NARRATIVES
```
Write in my voice as described in [STRATEGIES]. You may reference
personal experiences from [NARRATIVES]. Content must align with
my mission of [MISSION].
```

**Problem solving**: Inject CHALLENGES + LEARNED + PROJECTS
```
I'm facing [CHALLENGE] in the context of [PROJECT]. Past lessons:
[LEARNED]. Help me find an approach I haven't tried yet.
```

## Multi-Model Strategy

When the user has access to multiple LLMs:
- **Large context models**: Use for tasks requiring full TELOS injection (research, strategy)
- **Small fast models**: Use for tasks where MISSION + GOALS summary suffices (quick questions)
- **Local models**: Use when PRIVATE-tagged content needs to be included
- Always match TELOS injection depth to the model's context window

## Anti-Pattern Detection

Flag when a prompt would be improved by TELOS context:
- User asks for a recommendation without stating their goals → suggest GOALS injection
- User asks for content without stating their voice → suggest STRATEGIES injection
- User asks for analysis without stating their framework → suggest MODELS injection
