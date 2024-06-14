---
title: Upstream version {{ env.RELEASE_TAG }} released
labels: automated-issue
---

A new Aptos PFN release is available at {{ env.UPSTREAM_URL}}.

A new branch associated with these changes was pushed locally at `{{ env.RELEASE_TAG }}`.

Steps to release an upgraded patched version:
- Checkout a copy of `{{ env.RELEASE_TAG }}` as `release/{{ env.RELEASE_TAG }}-patched` to prepare for release
- Apply the patched changes to this branch with e.g. `git rebase --onto` or `git cherry-pick`. This is a manual process as it often creates conflicts. TODO document the full command, and do we need to explicitly tag the `-patched` branch or will the release job tag it with the  `softprops` workflow? 
- Then, run the release workflow at {{ env.RELEASE_PR_WORKFLOW }}. This will bump the version number in `README.md` (since there is no Cargo version for the Aptos node) and open a PR to `release/{{ env.RELEASE_TAG }}-patched` for the release. The PR will run CI checks, solicit review, and provide an artifact for downstream companion PRs to test on.
- When the PR is merged, it will automatically publish a GitHub release for `{{ env.RELEASE_TAG }}-patched` using `{{ env.RELEASE_MERGE_WORKFLOW }}`.

This issue was created by the workflow at {{ env.WORKFLOW_URL }}

