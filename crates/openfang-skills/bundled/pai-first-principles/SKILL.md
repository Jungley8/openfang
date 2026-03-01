---
name: pai-first-principles
description: "First principles reasoning using the user's own mental models, beliefs, and thinking frameworks from their TELOS profile"
---
# First Principles Reasoning with TELOS

Use the user's own mental models (MODELS.md) and beliefs (BELIEFS.md) as the foundation for first principles analysis, rather than generic frameworks.

## Reasoning Process

### Step 1: Identify the User's Axioms
From BELIEFS.md and MODELS.md, extract the user's foundational assumptions:
- What does the user consider to be true without needing proof?
- What frameworks does the user prefer for analyzing problems?
- What values constrain acceptable solutions?

### Step 2: Decompose the Problem
Break the question into sub-questions, working backward from the user's goals:
```
Question: [User's question]
Relevant goal: [From GOALS.md]
Relevant model: [From MODELS.md]

Sub-questions (derived from first principles):
1. What must be true for [goal] to succeed?
2. Which of these prerequisites does [question] address?
3. What are the irreducible components?
4. Which assumptions can be challenged?
```

### Step 3: Rebuild from Ground Truth
Using only verified facts and the user's axioms:
- Start with what is known to be true
- Build up one logical step at a time
- Flag where the reasoning depends on an assumption (not a fact)
- Note where the user's models provide specific guidance

### Step 4: Alignment Check
- Does the conclusion advance any active goal in GOALS.md?
- Does the reasoning conflict with any belief in BELIEFS.md?
- Has the user learned something relevant from LEARNED.md?
- Does this connect to a current challenge in CHALLENGES.md?

## Mental Model Application

When MODELS.md specifies frameworks, apply them explicitly:

| If User's Model Includes | Apply As |
|--------------------------|----------|
| First Principles (Musk) | Decompose to physics-level truths, rebuild |
| Inversion (Munger) | Ask "What would guarantee failure?" and avoid that |
| Second-Order Thinking | Trace consequences 2-3 steps ahead |
| Opportunity Cost | Compare against the next best alternative |
| Circle of Competence | Flag when analysis leaves the user's expertise domain |
| Pareto (80/20) | Identify the 20% of factors driving 80% of the outcome |

## Output Format

```markdown
## First Principles Analysis: [Question]

### Axioms Used
- [From BELIEFS.md]: ...
- [From MODELS.md]: ...

### Decomposition
[Sub-questions and irreducible components]

### Reasoning Chain
[Step-by-step from ground truth to conclusion]

### Conclusion
[Answer, with confidence level]

### Goal Alignment
[How this connects to GOALS.md]
```
