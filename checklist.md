# Checklist

## Pre-Commit Checklist
- [ ] Code compiles cleanly.
- [ ] No new lint errors.
- [ ] Tests pass for affected files.
- [ ] Documentation related to the change is updated.
- [ ] Backup created in `_backups/` before editing.
- [ ] No TODO/FIXME placeholders remain.
- [ ] Changelog entry drafted.

## Pre-PR Checklist
- [ ] PR title follows branch naming and commit conventions.
- [ ] Description includes the problem, fix, and docs updated.
- [ ] Screenshots or logs included if UI/behavior changed.
- [ ] Related docs links are listed.
- [ ] Backup and revert notes are included.

## Pre-Deploy Checklist
- [ ] Build passes in the target environment.
- [ ] Database schema and migrations are verified.
- [ ] Environment variables are documented.
- [ ] Rollback plan is defined.
- [ ] Post-deploy validation steps are documented.

## Accessibility Checklist
- [ ] Keyboard navigation works for all new UI.
- [ ] ARIA roles and labels are present.
- [ ] Contrast ratios meet WCAG AA.
- [ ] Focus styles are visible.
- [ ] Screen reader text is accurate.

## Security Checklist
- [ ] No new broad permissions are added.
- [ ] Data writes are validated before exec.
- [ ] User-selected paths are handled safely.
- [ ] Secrets are not stored in source files.
- [ ] Third-party dependencies are vetted.

## Post-Deploy Validation
- [ ] Verify critical flows manually.
- [ ] Confirm analytics/logging behavior.
- [ ] Check for regressions in the changed area.
- [ ] Confirm that docs and changelog reflect the deployed version.
