---
title: "chore: Update to upstream release `{{ env.RELEASE_TAG }}`"
labels: automated-issue
---

A new Aptos PFN release `{{ env.RELEASE_TAG }}` is available at {{ env.UPSTREAM_URL }}.

A new branch associated with these changes was pushed locally at `upstream/{{ env.RELEASE_TAG }}`.

Steps to release an upgraded patched version:
- Checkout a copy of `upstream/{{ env.RELEASE_TAG }}` as `release/{{ env.RELEASE_TAG }}-patched` to prepare for release
- Apply the patched changes to this branch with the following:
```
git cherry-pick 7c9a8bcd79376cf1479a3432b48127a56945cabb
git cherry-pick d14dc0c286e704883ca453a40ab4531702c86f71
git cherry-pick 15f91a32989ea63224064862167136b17baf1128
git push origin release/{{ env.RELEASE_TAG }}-patched
```
Note: This is a manual process as it often creates conflicts. TODO Do we need to explicitly tag the `-patched` branch or will the release job tag it with the  `softprops` workflow?
- Then, run the [release workflow]({{ env.RELEASE_PR_WORKFLOW }}) and set the version input to `{{ env.RELEASE_TAG }}`. This will bump the version number in `README.md` (since there is no Cargo version for the Aptos node) and open a PR from `release/{{ env.RELEASE_TAG }}-patched` to the `release` branch. The PR will run CI checks, solicit review, and provide an artifact for downstream companion PRs to test on.
- When the PR is merged, it will automatically publish a GitHub release for `{{ env.RELEASE_TAG }}-patched` using the [merge workflow]({{ env.RELEASE_MERGE_WORKFLOW }}).

This issue was created by the workflow at {{ env.WORKFLOW_URL }}

TODO: Move these instructions to separate patch-notes.md file and link to it here

