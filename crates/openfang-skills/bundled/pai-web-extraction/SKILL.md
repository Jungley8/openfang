---
name: pai-web-extraction
description: "TELOS-aligned web data extraction — browser automation patterns guided by user goals, ethical boundaries from beliefs, and purchase authorization from objectives"
---
# TELOS-Aligned Web Operations

Browser operations should be guided by the user's TELOS context for targeting, ethical constraints, and authorization boundaries.

## TELOS-Guided Navigation

### Target Selection
- Derive URLs and search targets from PROJECTS.md (competitor sites, tech docs)
- Use GOALS.md to prioritize which data to collect first
- Check CHALLENGES.md for specific data gaps to fill

### Ethical Boundaries (from BELIEFS.md)
Before executing any web action, verify:
1. Does this action conflict with any stated belief or value?
2. Would the user be comfortable if this action were public?
3. Common ethical constraints to check:
   - No scraping of personal data without consent
   - No impersonation or social engineering
   - No circumventing paywalls or access controls
   - Respect robots.txt and rate limits

### Purchase/Spend Authorization
Any action involving money must:
- Link to a specific goal in GOALS.md ("Why is this purchase necessary?")
- Stay within any stated budget constraints
- Enter the approval queue — never auto-purchase

## Data Extraction Patterns

### Competitive Intelligence Extraction
```
For each competitor in PROJECTS.md:
1. Pricing pages → extract plan structure, feature matrix
2. Changelog/blog → extract recent product updates
3. Job postings → infer team focus and growth areas
4. Social profiles → extract engagement metrics and content strategy
```

### Market Research Extraction
```
For each goal in GOALS.md:
1. Industry report sites → extract market size, growth data
2. Review platforms → extract user sentiment for your category
3. Investor databases → extract funding trends in your space
```

## Output Formatting
- Always cite the source URL for extracted data
- Include extraction timestamp (web data goes stale fast)
- Flag data confidence: verified (from primary source) vs inferred (from secondary)
- Structure output to directly answer TELOS-derived questions
