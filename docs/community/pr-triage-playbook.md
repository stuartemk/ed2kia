# PR Triage Playbook — ed2kIA v1.8

## Purpose

Standardize the process for reviewing, categorizing, and responding to community Pull Requests. This playbook ensures consistent, timely, and transparent PR handling across all ed2kIA repositories.

## Triage Workflow

### 1. Initial Review (Within 24h)

- [ ] Check PR title follows conventional commits: `type(scope): description`
- [ ] Verify PR description is complete (problem, solution, testing)
- [ ] Check CI status (tests, clippy, check)
- [ ] Assign appropriate labels

### 2. Categorization

| Category | Labels | Response Time | Action |
|----------|--------|---------------|--------|
| **Critical Bug Fix** | `bug`, `priority::critical` | < 4h | Review → Merge or request changes |
| **Security Fix** | `security`, `priority::critical` | < 2h | Review → Merge immediately |
| **Feature** | `feature` | < 48h | Review → Request changes or approve |
| **Documentation** | `documentation` | < 72h | Review → Merge or suggest edits |
| **Refactor** | `refactor` | < 72h | Review → Discuss impact |
| **CI/Build** | `ci`, `build` | < 48h | Review → Test locally → Merge |
| **Question/Discussion** | `question` | < 48h | Comment → Resolve or close |

### 3. Review Checklist

- [ ] Code follows project style (clippy clean)
- [ ] Tests included for new functionality
- [ ] Documentation updated if API changed
- [ ] No breaking changes without deprecation notice
- [ ] Feature flags used appropriately
- [ ] CHANGELOG updated if applicable

### 4. Response Templates

#### Approved
```
LGTM! Changes look solid. A few minor suggestions:
- [suggestion 1]
- [suggestion 2]

Once addressed, this is ready to merge.
```

#### Needs Work
```
Thanks for the contribution! Here are the areas that need attention:
1. [Issue 1] — [Explanation]
2. [Issue 2] — [Explanation]

Please address these and re-request review.
```

#### Out of Scope
```
Thanks for the interest in ed2kIA! This change is currently out of scope
because [reason]. We suggest [alternative approach].

Feel free to open an issue to discuss further.
```

## Automation

- **Labels**: Auto-applied via `.github/workflows/pr-labeler.yml`
- **Assignments**: Auto-assigned to maintainers based on file paths
- **Stale PRs**: Auto-closed after 30 days of inactivity
- **First-time contributors**: Auto-welcomed via PR comments

## Escalation Path

1. **Maintainer Review** → 2. **Tech Lead Approval** → 3. **Core Team Decision**

For security issues, follow the [Security Disclosure Policy](../SECURITY_DISCLOSURE.md).

## Metrics

Track these metrics weekly:
- Average time to first response
- Average time to merge
- PR acceptance rate
- Contributor retention rate

## References

- [Contributing Guide](../../CONTRIBUTING.md)
- [First Contributor Guide](first-contributor-guide.md)
- [Community Funding](../community_funding_v1.3.md)
