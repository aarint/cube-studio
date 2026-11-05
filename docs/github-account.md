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

## Releasing

Releases are published by `.github/workflows/release.yml`, which runs as the
repository owner. No personal access token is needed for CI; the workflow's own
`GITHUB_TOKEN` has `contents: write` for creating the release.

Pushing a tag publishes immediately. To test the pipeline without publishing,
use **Actions → release → Run workflow** — that path builds and uploads artefacts
but never touches a release.
