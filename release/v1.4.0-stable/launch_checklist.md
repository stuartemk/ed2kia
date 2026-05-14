# ed2kIA v1.4.0 STABLE — Launch Checklist

## Pre-Launch Validation

### Compilation & Quality Gates
- [x] `cargo check --features stable` — 0 errors
- [x] `cargo clippy --features stable -- -D warnings` — 0 errors, 0 warnings
- [x] `cargo test --features stable --no-run` — 0 errors
- [x] Legacy tests (v1.1-sprint4) excluded from stable feature
- [x] Duplicate mod declarations fixed (sae_v2)

### Guardrails
- [x] Zero financial logic
- [x] Zero telemetry
- [x] Apache 2.0 + Ethical Use license
- [x] Zero unsafe blocks
- [x] All code feature-gated under `stable`

### Test Coverage
- [x] SAE Fine-Tuning v4: 73 tests
- [x] Federation Scaling v4: 67 tests
- [x] Async ZKP v8: 56 tests
- [x] Cross-Federation Verification: 56 tests
- [x] Sprint 3 E2E: 12 tests
- [x] Sprint 3 Stress: 7 tests
- [x] **Total: 213+ tests passing**

## Release Artifacts

### Packaging
- [ ] `bash release/v1.4.0-stable/package_release.sh`
- [ ] Verify `release/v1.4.0-stable/bin/ed2kia` exists
- [ ] Verify `release/v1.4.0-stable/checksums.sha256`
- [ ] Verify `release/v1.4.0-stable/MANIFEST.json`

### Documentation
- [ ] `docs/official_launch_announcement_v1.4.md`
- [ ] `docs/migration_guide_v1.3_to_v1.4.md`
- [ ] `docs/architecture_v1.4.0.md`
- [ ] `release/v1.4.0-stable/final_validation_report.json`

### Signing
- [ ] `gpg --detach-sign release/v1.4.0-stable/bin/ed2kia`
- [ ] Verify signature: `gpg --verify release/v1.4.0-stable/bin/ed2kia.sig`

## Deployment

### Docker
- [ ] Build: `docker build -t ed2kia:v1.4.0 -f deploy/Dockerfile .`
- [ ] Test: `docker run --rm ed2kia:v1.4.0 --version`
- [ ] Push to registry

### Systemd
- [ ] Verify `deploy/systemd/ed2kia.service`
- [ ] Verify `deploy/systemd/ed2kia.env`
- [ ] Test install: `bash deploy/systemd/install.sh`

### Git Tag
- [ ] `git tag -a v1.4.0 -m 'Release v1.4.0 STABLE'`
- [ ] `git push origin v1.4.0`

## Post-Launch

- [ ] Monitor node health for 24h
- [ ] Verify P2P connectivity
- [ ] Check dashboard metrics
- [ ] Community announcement
- [ ] Update CONTRIBUTING.md with v1.4.0 references

---

**Version:** v1.4.0 STABLE
**Date:** 2026-05-11
**License:** Apache 2.0 + Ethical Use
