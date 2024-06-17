---
title: "chore: Update to upstream release `{{ env.RELEASE_TAG }}`"
labels: automated-issue
---

A new Aptos PFN release `{{ env.RELEASE_TAG }}` is available at {{ env.UPSTREAM_URL }}.

A new branch associated with these changes was pushed locally at `upstream/{{ env.RELEASE_TAG }}`.

Steps to release an upgraded patched version:
- From `dev`, create a new branch `{{ env.RELEASE_TAG }}-patched`
- Pull the changes from `upstream/{{ env.RELEASE_TAG }}` as follows:
```
- Apply the patched changes to this branch with the following:
```
git remote add upstream https://github.com/aptos-labs/aptos-core.git
git pull upstream refs/tags/{{ env.RELEASE_TAG }} -r
git push origin {{ env.RELEASE_TAG }}-patched
```
- Then open a PR to `dev` with the changes, ensuring CI passes as usual
- Then, once this PR is merged, run the [release workflow]({{ env.RELEASE_PR_WORKFLOW }}) and set the version input to `{{ env.RELEASE_TAG }}`. This will bump the version number in `PATCH_RELEASE.md` (since there is no Cargo version for the Aptos node) and open a PR from `release/{{ env.RELEASE_TAG }}-patched` to `dev`. The PR will run CI checks and provide an artifact for downstream companion PRs to test on. Note: This is a manual process as the rebase will likely create conflicts that must be resolved manually.
- When the PR is merged, it will automatically publish a GitHub release for `{{ env.RELEASE_TAG }}-patched` using the [merge workflow]({{ env.RELEASE_MERGE_WORKFLOW }}).

This issue was created by the workflow at {{ env.WORKFLOW_URL }}

TODO: Move these instructions to separate patch-notes.md file and link to it here
TODO Do we need to explicitly tag the `-patched` branch or will the release job tag it with the  `softprops` workflow?
