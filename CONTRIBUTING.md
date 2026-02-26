# Contributing

## Documentation Workflow

We aim to keep documentation in sync with code. To achieve this without blocking PRs, we use the following workflow:

1. **When making code changes**: Apply the `docs:pending` label to your PR
2. **Manual bulk processing**: Maintainers periodically review **closed** PRs with the `docs:pending` label and update docs in batches
3. **Completion label**: After docs are updated for a PR, add `docs:updated`
4. **Cleanup**: Remove `docs:pending` after adding `docs:updated`

This approach allows code changes to land quickly while ensuring documentation stays current.

### Maintainer Bulk Run Checklist

Use the GitHub CLI to process closed PRs that still have `docs:pending`.

1. **List candidates**
	```bash
	gh pr list --state closed --search 'label:"docs:pending"' --limit 200
	```
2. **Update documentation in bulk** based on those PRs.
3. **Mark PRs completed** (add `docs:updated`, remove `docs:pending`):
	```bash
	gh pr list --state closed --search 'label:"docs:pending"' --json number --jq '.[].number' | \
	while read -r pr; do
	  gh pr edit "$pr" --add-label 'docs:updated' --remove-label 'docs:pending'
	done
	```
4. **Verify no pending closed PRs remain**
	```bash
	gh pr list --state closed --search 'label:"docs:pending"' --limit 50
	```
