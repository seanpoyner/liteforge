#!/usr/bin/env bash
#
# sync-wiki.sh — publish wiki/ (the source of truth) to the GitHub wiki repo.
#
# The wiki's canonical source lives in wiki/ in this repo so it gets version
# control and PR review. This script mirrors those files into the GitHub wiki
# repository (github.com/<owner>/<repo>.wiki.git) and pushes.
#
# Auth: uses the `gh` CLI credential helper — no token is read, printed, or
# embedded. Make sure you're logged in:  gh auth status
#
# Usage:
#   scripts/sync-wiki.sh            # sync to the default repo (seanpoyner/liteforge)
#   REPO=owner/name scripts/sync-wiki.sh
#   DRY_RUN=1 scripts/sync-wiki.sh  # show what would change, don't push
#
set -euo pipefail

REPO="${REPO:-seanpoyner/liteforge}"
WIKI_URL="https://github.com/${REPO}.wiki.git"
WIKI_WEB="https://github.com/${REPO}/wiki"

# Resolve repo root (this script lives in <root>/scripts/).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
SRC_DIR="${ROOT_DIR}/wiki"
WORK_DIR="${ROOT_DIR}/.wiki-sync"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

command -v git >/dev/null 2>&1 || die "git is required"
command -v gh  >/dev/null 2>&1 || die "the GitHub CLI (gh) is required for auth — install it or run: gh auth login"
[[ -d "${SRC_DIR}" ]] || die "no wiki/ directory at ${SRC_DIR}"
gh auth status >/dev/null 2>&1 || die "not logged in to gh — run: gh auth login"

# Authenticate git over HTTPS via gh, without exposing a token.
GIT_AUTH=(-c "credential.helper=" -c "credential.helper=!gh auth git-credential")

echo "==> Syncing ${SRC_DIR}  ->  ${WIKI_URL}"

# Fresh working clone of the wiki repo.
rm -rf "${WORK_DIR}"
if git "${GIT_AUTH[@]}" clone --quiet "${WIKI_URL}" "${WORK_DIR}" 2>/dev/null; then
    echo "==> Cloned existing wiki repo"
else
    echo "==> Wiki repo not clonable yet — initializing a new one"
    echo "    (if the push below fails with 'Repository not found', create the first"
    echo "     page once at ${WIKI_WEB} then re-run this script — see Contributing)"
    git init --quiet "${WORK_DIR}"
    git -C "${WORK_DIR}" remote add origin "${WIKI_URL}"
fi

# Mirror the markdown pages. Delete pages that no longer exist in wiki/,
# but never touch the wiki repo's .git directory.
shopt -s nullglob
for f in "${WORK_DIR}"/*.md; do rm -f "$f"; done
cp "${SRC_DIR}"/*.md "${WORK_DIR}/"
# wiki/README.md documents the source dir; it isn't a wiki page — drop it.
rm -f "${WORK_DIR}/README.md"
shopt -u nullglob

cd "${WORK_DIR}"
git add -A

if git diff --cached --quiet; then
    echo "==> No changes to publish — wiki is already up to date."
    cd "${ROOT_DIR}"; rm -rf "${WORK_DIR}"
    exit 0
fi

echo "==> Pages to publish:"
git diff --cached --name-status | sed 's/^/    /'

if [[ -n "${DRY_RUN:-}" ]]; then
    echo "==> DRY_RUN set — not committing or pushing."
    cd "${ROOT_DIR}"; rm -rf "${WORK_DIR}"
    exit 0
fi

git -c user.name="liteforge-wiki-sync" \
    -c user.email="wiki-sync@users.noreply.github.com" \
    commit --quiet -m "docs: sync wiki from wiki/"

# Push to whichever default branch the wiki uses (master for GitHub wikis).
BRANCH="$(git symbolic-ref --quiet --short HEAD || echo master)"
if git "${GIT_AUTH[@]}" push --quiet origin "HEAD:${BRANCH}" 2>/dev/null \
   || git "${GIT_AUTH[@]}" push --quiet origin "HEAD:master" 2>/dev/null \
   || git "${GIT_AUTH[@]}" push --quiet origin "HEAD:main" 2>/dev/null; then
    echo "==> Published. View it at ${WIKI_WEB}"
else
    die "push failed. If this is a brand-new wiki, create the first page once at ${WIKI_WEB} (click 'Create the first page'), then re-run this script."
fi

cd "${ROOT_DIR}"
rm -rf "${WORK_DIR}"
