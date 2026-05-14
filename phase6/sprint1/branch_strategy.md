# Phase 6 - Branch Strategy & Git Workflow

## Branch Model

```
main (v0.5.0 stable)
  │
  ├── release/v0.6.0 (integration)
  │     │
  │     └── dev/fase6 (development)
  │           │
  │           ├── feat/interoperability-adapter
  │           ├── feat/fedavg-krum
  │           ├── feat/staking-registry
  │           ├── feat/api-openapi
  │           ├── fix/consensus-timeout
  │           └── docs/api-reference
  │
  └── hotfix/v0.5.1 (production fixes only)
```

## Branch Descriptions

| Branch | Purpose | Protected | Lifecycle |
|--------|---------|-----------|-----------|
| `main` | Production stable (v0.5.0) | ✅ Yes | Permanent |
| `release/v0.6.0` | Phase 6 integration testing | ✅ Yes | Until v0.6.0 release |
| `dev/fase6` | Active Phase 6 development | ⚠️ Semi | Until v0.6.0 release |
| `feat/*` | Individual features | ❌ No | Deleted after merge |
| `fix/*` | Bug fixes | ❌ No | Deleted after merge |
| `docs/*` | Documentation changes | ❌ No | Deleted after merge |
| `chore/*` | Build/CI/tooling changes | ❌ No | Deleted after merge |
| `hotfix/*` | Production emergency fixes | ✅ Yes | Merged to main + dev/fase6, deleted |

---

## Protection Rules

### `main` (Production)

- ✅ Require pull request before merging
- ✅ Require 2 approving reviews
- ✅ Require status checks to pass (CI)
- ✅ Require branch to be up to date
- ✅ Include administrators in rules
- ❌ Do not allow force pushes
- ❌ Do not allow deletions
- ✅ Require signed commits (DCO)

### `release/v0.6.0` (Integration)

- ✅ Require pull request before merging
- ✅ Require 1 approving review
- ✅ Require status checks to pass
- ✅ Require branch to be up to date
- ❌ Do not allow force pushes

### `dev/fase6` (Development)

- ✅ Require status checks to pass
- ⚠️ Force pushes allowed (maintainers only)
- ❌ No required reviews (encouraged)

---

## Commit Conventions

### Format

```
type(scope): short description

Longer description if needed. Explain what and why, not how.

- Bullet points for details
- Reference issues: Closes #123

Co-Authored-By: Name <email>
Signed-off-by: Author <email>
```

### Types

| Type | Description | Example |
|------|-------------|---------|
| `feat` | New feature | `feat(adapter): add Llama-3 hidden state extraction` |
| `fix` | Bug fix | `fix(krum): handle empty update list` |
| `docs` | Documentation | `docs(api): add OpenAPI spec for /v2/models` |
| `style` | Code style (formatting, no logic) | `style(federation): format with rustfmt` |
| `refactor` | Code change (no feature, no fix) | `refactor(staking): extract proof validation` |
| `test` | Add or update tests | `test(fedavg): add Byzantine tolerance tests` |
| `chore` | Build, CI, dependencies | `chore(ci): add phase6-experimental feature flag` |
| `perf` | Performance improvement | `perf(krum): parallelize distance calculations` |

### Scopes

| Scope | Module |
|-------|--------|
| `adapter` | `src/interoperability/adapter.rs` |
| `schema` | `src/interoperability/schema.rs` |
| `fedavg` | `src/federation/avg_aggregator.rs` |
| `sync` | `src/federation/sync_protocol.rs` |
| `staking` | `src/staking/registry.rs` |
| `proof` | `src/staking/proof.rs` |
| `api` | `src/api/openapi.rs`, `src/api/routes.rs` |
| `build` | Build system, Cargo.toml |
| `ci` | CI/CD pipelines |
| `deps` | Dependency updates |

### Examples

```bash
# Good commits
git commit -m "feat(adapter): add Llama-3 hidden state extraction

Implement ModelAdapter trait for Llama-3 models.
Extracts hidden states from layer -2 and maps to SAE input format.

Closes #601"

git commit -m "fix(krum): handle empty update list in aggregation

Return error when no updates available instead of panicking.

Fixes #615"

git commit -m "test(fedavg): add Byzantine tolerance test suite

Test cases for 0, 1, 2, and 3 Byzantine nodes in n=10 network.
Verifies Krum correctly filters outliers when f < n/3."

# Bad commits
git commit -m "fix stuff"              # Too vague
git commit -m "WIP"                    # Never commit WIP
git commit -m "updated code"           # No context
```

---

## Pull Request Workflow

### Creating a PR

1. Push feature branch to origin
2. Create PR targeting `dev/fase6`
3. Fill out PR template completely
4. Request review from maintainers
5. Address feedback iteratively
6. Squash merge after approval

### PR Template Checklist

```markdown
## Description
<!-- What does this PR do? -->

## Type
- [ ] feat
- [ ] fix
- [ ] docs
- [ ] test
- [ ] chore

## Checklist
- [ ] cargo check --features "phase6-experimental" passes
- [ ] cargo clippy --features "phase6-experimental" clean
- [ ] cargo test --features "phase6-experimental" passes
- [ ] Documentation updated
- [ ] DCO sign-off included

## Related Issues
Closes #XXX
```

### Merge Criteria

| Target | Reviews | CI | DCO |
|--------|---------|-----|-----|
| `dev/fase6` | 0 (encouraged: 1) | ✅ Required | ✅ Required |
| `release/v0.6.0` | 1 | ✅ Required | ✅ Required |
| `main` | 2 | ✅ Required | ✅ Required |

---

## DCO (Developer Certificate of Origin)

Sign off commits with:
```bash
git commit -s -m "feat(adapter): add Llama-3 support"
```

This adds: `Signed-off-by: Your Name <your@email.com>`

### DCO Statement

By signing off, you certify:
- You wrote the code or have permission to contribute it
- You agree to the Apache 2.0 license
- You respect the Ethical Use Clause

---

## CI Pipeline

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check --features "phase6-experimental"

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - run: cargo clippy --features "phase6-experimental" -- -D warnings

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --features "phase6-experimental"

  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt -- --check
```

---

## Release Workflow

### From dev/fase6 to release/v0.6.0

```bash
# 1. Create merge PR
git checkout release/v0.6.0
git merge dev/fase6 --no-ff -m "merge: integrate dev/fase6 into release/v0.6.0"

# 2. Run full test suite
cargo test --features "phase6-experimental"

# 3. Push and create PR on GitHub
git push origin release/v0.6.0
```

### From release/v0.6.0 to main (v0.6.0 release)

```bash
# 1. Update version in Cargo.toml
# 2. Generate release notes
# 3. Create tag
git tag -a v0.6.0 -m "ed2kIA v0.6.0 - Phase 6 Release"

# 4. Merge to main
git checkout main
git merge release/v0.6.0 --no-ff
git push origin main --tags
```

---

## Hotfix Procedure

```bash
# 1. Create hotfix branch from main
git checkout main
git checkout -b hotfix/v0.5.1

# 2. Fix the issue
# ... make changes ...

# 3. Test thoroughly
cargo test --features "core-only"

# 4. Merge to main with PR (2 approvals required)
git push origin hotfix/v0.5.1

# 5. Back-port to dev/fase6
git checkout dev/fase6
git cherry-pick <hotfix-commit>

# 6. Delete hotfix branch
git branch -d hotfix/v0.5.1
```
