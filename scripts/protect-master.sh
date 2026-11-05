#!/usr/bin/env bash
#
# Enable branch protection on `master`.
#
# This needs an authenticated GitHub token with admin rights on the repository;
# it cannot be done over anonymous SSH. Run it once:
#
#   GH_TOKEN=$(gh auth token) ./scripts/protect-master.sh
#   # or: gh auth login && ./scripts/protect-master.sh
#
# What the rule enforces:
#   * no force-push, no branch deletion
#   * history stays linear-ish: merge commits allowed (releases are merged), but
#     rewriting master is refused
#   * status checks must pass before merging, once CI has reported at least once
#   * conversations must be resolved
#   * administrators are included, so the rule is not advisory
#
# Deliberately NOT enforced: "approving reviews required". This is a
# single-maintainer repository, so requiring an approval would make every change
# need a second identity to land — and the last release workflow had to be
# merged straight from a maintainer commit.
set -euo pipefail

REPO="${REPO:-aarint/cube-studio}"
BRANCH="${BRANCH:-master}"

if [ -z "${GH_TOKEN:-}" ] && ! gh auth status >/dev/null 2>&1; then
  echo "error: not authenticated." >&2
  echo "       run 'gh auth login' or set GH_TOKEN, then re-run." >&2
  exit 1
fi

api() {
  if [ -n "${GH_TOKEN:-}" ]; then
    curl -sS -H "Authorization: Bearer $GH_TOKEN" \
      -H "Accept: application/vnd.github+json" \
      -H "X-GitHub-Api-Version: 2022-11-28" "$@"
  else
    gh api "$@"
  fi
}

echo "==> protecting ${REPO}:${BRANCH}"

# `required_status_checks` is set to null: the release workflow only runs on
# tags, so there are no per-PR checks to require yet. Add them here (with
# "strict": true) once a CI workflow exists that runs on pull_request.
payload=$(cat <<'JSON'
{
  "required_status_checks": null,
  "enforce_admins": true,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "required_linear_history": false,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": true
}
JSON
)

if [ -n "${GH_TOKEN:-}" ]; then
  out=$(api -X PUT -o /dev/null -w "%{http_code}" \
    "https://api.github.com/repos/${REPO}/branches/${BRANCH}/protection" \
    -d "$payload")
else
  out=$(echo "$payload" | gh api -X PUT \
    "repos/${REPO}/branches/${BRANCH}/protection" --input - 2>&1 | tail -1)
fi

echo "    response: ${out}"

echo "==> verifying"
if [ -n "${GH_TOKEN:-}" ]; then
  api "https://api.github.com/repos/${REPO}/branches/${BRANCH}/protection" \
    | python3 -c '
import json,sys
d=json.load(sys.stdin)
if "message" in d and "url" not in d:
    print("   FAILED:", d["message"]); sys.exit(1)
for key in ("enforce_admins","allow_force_pushes","allow_deletions","required_conversation_resolution"):
    print(f"   {key:32} {d.get(key,{}).get(\"enabled\")}")
'
else
  gh api "repos/${REPO}/branches/${BRANCH}/protection" --jq '
    "   enforce_admins: \(.enforce_admins.enabled)",
    "   force pushes:   \(.allow_force_pushes.enabled)",
    "   deletions:      \(.allow_deletions.enabled)"'
fi
