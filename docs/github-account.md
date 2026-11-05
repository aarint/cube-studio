# This repository is owned by the `aarint` GitHub account.

The local SSH agent holds a key for a *different* GitHub account (`aarcore`), so
`git@github.com:` would authenticate as the wrong user. Two settings keep every
git operation on `aarint`, and they are committed here rather than left to the
machine's global config so a fresh clone behaves the same way.

```
# remote
origin  git@aarint.com:aarint/cube-studio.git

# per-repo override (set automatically on clone checkouts that include this note)
core.sshCommand = ssh -i ~/.ssh/id_ed25519_aarint -o IdentitiesOnly=yes
```

`IdentitiesOnly=yes` is the important part: without it, ssh-agent offers every
loaded key and GitHub authenticates whichever account is matched first — which is
how the `aarcore` key was winning.

## Verify before pushing

```bash
ssh -i ~/.ssh/id_ed25519_aarint -o IdentitiesOnly=yes -T git@github.com
# expected: Hi aarint! You've successfully authenticated...
```

```bash
git config user.name   # aarint
git config user.email  # aardoit@gmail.com
git log -1 --format='%an <%ae>'
```

## Branch model

`master` is the default and protected branch, and every branch is merged into
it. The two older branches are kept only as release history:

| Branch | State |
| --- | --- |
| `master` | default, protected, level with `1.1.0` |
| `1.1.0` | fully merged into `master`; no unique commits |
| `1.0.0` | fully merged into `master`; no unique commits |

Enable the protection rule once, from a machine with a token that has admin
rights on the repository:

```bash
gh auth login                      # or export GH_TOKEN=...
./scripts/protect-master.sh
```

That rule blocks force-pushes and branch deletion, requires conversations to be
resolved, and applies to administrators too. It deliberately does not require an
approving review: this is a single-maintainer repository, so a mandatory second
approval would block every change.

## Releasing

Releases are published by `.github/workflows/release.yml`, which runs as the
repository owner. No personal access token is needed for CI; the workflow's own
`GITHUB_TOKEN` has `contents: write` for creating the release.

Pushing a tag publishes immediately. To test the pipeline without publishing,
use **Actions → release → Run workflow** — that path builds and uploads artefacts
but never touches a release.
