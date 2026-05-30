#!/usr/bin/env bash
#
# ono-sendai single-file release script.
#
# Purpose:
#   Take the working tree from "committed on main, CI green" all the way
#   to "tag pushed, GitHub release created, all 8 library crates + the
#   binary published to crates.io, repo metadata correct".
#
# Design notes:
#   - Idempotent. Re-running after a partial failure resumes where the
#     previous run stopped (each phase checks its own completion marker).
#   - Defaults to --dry-run mode so you can review intent. Pass
#     --execute to actually publish.
#   - The ONLY truly manual step is `cargo login <TOKEN>` (R11: secret
#     tokens never flow through the script's argv or env). If the
#     credentials file is absent the script exits early with a hint.
#
# Usage:
#   ./scripts/release.sh                    # dry-run (prints intent)
#   ./scripts/release.sh --execute          # actually publish
#   ./scripts/release.sh --execute 0.1.2    # bump first (rare path)
#   ./scripts/release.sh --skip-publish     # tag + GH release only,
#                                           # skip crates.io
#   ./scripts/release.sh --skip-community   # do not touch .github/
#   ./scripts/release.sh --skip-dry-run     # bypass Phase 4 (use when
#                                           # path-dep crates have not
#                                           # been published yet)
#
# Exit codes:
#   0  success (or dry-run completed)
#   1  preflight failure (auth, dirty tree, wrong branch, etc.)
#   2  user aborted at confirmation
#   3  cargo publish failed mid-stream — re-run to resume
#   4  unexpected error (set -e from an unguarded command)

set -Eeuo pipefail

# -----------------------------------------------------------------------------
# config
# -----------------------------------------------------------------------------

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DEFAULT_BRANCH="main"
REPO_SLUG="hinanohart/ono-sendai"

# Topological publish order. Each later crate may depend on any earlier
# one, but never the reverse. The binary crate (ono-sendai) is last.
PUBLISH_ORDER=(
  "crates/deck-core"
  "crates/deck-llm"
  "crates/deck-mcp"
  "crates/deck-store"
  "crates/deck-sandbox"
  "crates/deck-plugin"
  "crates/deck-orchestrator"
  "crates/deck-tui"
  "."
)

EXECUTE=0
SKIP_PUBLISH=0
SKIP_COMMUNITY=0
SKIP_DRY_RUN=0
ALLOW_TAG_MISMATCH=0
BUMP_VERSION=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --execute) EXECUTE=1; shift ;;
    --skip-publish) SKIP_PUBLISH=1; shift ;;
    --skip-community) SKIP_COMMUNITY=1; shift ;;
    --skip-dry-run) SKIP_DRY_RUN=1; shift ;;
    --allow-tag-mismatch) ALLOW_TAG_MISMATCH=1; shift ;;
    -h|--help)
      sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      if [[ -z "$BUMP_VERSION" && "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-.*)?$ ]]; then
        BUMP_VERSION="$1"
        shift
      else
        printf '\033[31munknown arg: %s\033[0m (see --help)\n' "$1" >&2
        exit 1
      fi
      ;;
  esac
done

# -----------------------------------------------------------------------------
# pretty-print helpers
# -----------------------------------------------------------------------------

c_reset='\033[0m'; c_bold='\033[1m'; c_dim='\033[2m'
c_green='\033[32m'; c_yellow='\033[33m'; c_red='\033[31m'; c_blue='\033[34m'

step() { printf "\n${c_bold}${c_blue}==> %s${c_reset}\n" "$*"; }
info() { printf "    ${c_dim}%s${c_reset}\n" "$*"; }
ok()   { printf "    ${c_green}ok${c_reset}: %s\n" "$*"; }
warn() { printf "    ${c_yellow}warn${c_reset}: %s\n" "$*" >&2; }
die()  { printf "    ${c_red}fatal${c_reset}: %s\n" "$*" >&2; exit "${2:-1}"; }

if [[ $EXECUTE -eq 0 ]]; then
  step "DRY RUN — no mutating commands will run. Pass --execute to actually publish."
fi

run() {
  # run COMMAND ...  — log it, then execute (or skip in dry-run).
  printf "    ${c_dim}\$${c_reset} %s\n" "$*"
  if [[ $EXECUTE -eq 1 ]]; then
    "$@"
  fi
}

# -----------------------------------------------------------------------------
# phase 0 — preflight
# -----------------------------------------------------------------------------

step "Phase 0: preflight"

command -v cargo >/dev/null || die "cargo not on PATH"
command -v gh    >/dev/null || die "gh CLI not on PATH (https://cli.github.com/)"
command -v jq    >/dev/null || die "jq not on PATH (apt install jq / brew install jq)"
command -v git   >/dev/null || die "git not on PATH"
ok "cargo, gh, jq, git on PATH"

gh auth status >/dev/null 2>&1 || die "gh not authenticated. Run: gh auth login"
ok "gh authenticated"

if [[ $SKIP_PUBLISH -eq 0 ]]; then
  CRED_FILE=""
  for f in "$HOME/.cargo/credentials.toml" "$HOME/.cargo/credentials"; do
    [[ -f "$f" ]] && CRED_FILE="$f" && break
  done
  if [[ -z "$CRED_FILE" ]]; then
    die "no ~/.cargo/credentials[.toml] found. REQUIRES: cargo login <crates.io-token>" 1
  fi
  ok "cargo credentials present at $CRED_FILE"
fi

CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[[ "$CURRENT_BRANCH" == "$DEFAULT_BRANCH" ]] \
  || die "expected branch '$DEFAULT_BRANCH', got '$CURRENT_BRANCH'. Switch first."
ok "on $DEFAULT_BRANCH"

if ! git diff-index --quiet HEAD --; then
  die "working tree is dirty. Commit or stash before releasing."
fi
ok "clean working tree"

git fetch origin "$DEFAULT_BRANCH" >/dev/null 2>&1 || warn "git fetch failed (offline?)"
LOCAL_HEAD="$(git rev-parse HEAD)"
REMOTE_HEAD="$(git rev-parse "origin/$DEFAULT_BRANCH" 2>/dev/null || echo unknown)"
if [[ "$LOCAL_HEAD" != "$REMOTE_HEAD" ]]; then
  warn "local HEAD ($LOCAL_HEAD) differs from origin/$DEFAULT_BRANCH ($REMOTE_HEAD)"
  warn "the release will tag local HEAD. If that's intended, push first or continue."
else
  ok "local HEAD synced with origin/$DEFAULT_BRANCH"
fi

# -----------------------------------------------------------------------------
# phase 1 — resolve target version
# -----------------------------------------------------------------------------

step "Phase 1: resolve version"

# Read current workspace.package version from Cargo.toml.
CURRENT_VERSION="$(awk -F\" '/^\[workspace\.package\]/{f=1} f && /^version[[:space:]]*=/{print $2; exit}' Cargo.toml)"
[[ -n "$CURRENT_VERSION" ]] || die "could not parse workspace.package.version from Cargo.toml"
info "Cargo.toml says version = $CURRENT_VERSION"

if [[ -n "$BUMP_VERSION" ]]; then
  TARGET_VERSION="$BUMP_VERSION"
  if [[ "$TARGET_VERSION" != "$CURRENT_VERSION" ]]; then
    step "Bumping workspace version: $CURRENT_VERSION -> $TARGET_VERSION"
    # workspace.package.version is the only [workspace.package] version
    # field; the path-dependency `version = "X.Y.Z"` lines also need to
    # match for crates.io.
    if [[ $EXECUTE -eq 1 ]]; then
      sed -i.bak \
        -e "s/^version = \"$CURRENT_VERSION\"/version = \"$TARGET_VERSION\"/" \
        Cargo.toml
      rm -f Cargo.toml.bak
      cargo update --workspace >/dev/null
      git add Cargo.toml Cargo.lock
      git commit -m "release: bump to $TARGET_VERSION"
      git push origin "$DEFAULT_BRANCH"
      ok "bumped and pushed"
    else
      info "(dry-run) would sed/commit/push the version bump"
    fi
  fi
else
  TARGET_VERSION="$CURRENT_VERSION"
fi
TAG="v$TARGET_VERSION"
ok "target version: $TARGET_VERSION  (tag: $TAG)"

# -----------------------------------------------------------------------------
# phase 2 — community files sanity (optional)
# -----------------------------------------------------------------------------

if [[ $SKIP_COMMUNITY -eq 0 ]]; then
  step "Phase 2: community files sanity"
  for f in CODE_OF_CONDUCT.md SECURITY.md CONTRIBUTING.md LICENSE-APACHE LICENSE-MIT \
           .github/dependabot.yml .github/ISSUE_TEMPLATE/bug.yml \
           .github/ISSUE_TEMPLATE/feature.yml .github/PULL_REQUEST_TEMPLATE.md; do
    if [[ -f "$f" ]]; then
      ok "$f present"
    else
      warn "$f missing — re-run with --skip-community to suppress, or add the file"
    fi
  done
  for p in plugins/icebreaker/Cargo.toml plugins/mesh/Cargo.toml; do
    if grep -q '^publish = false' "$p"; then
      ok "$p has publish = false"
    else
      die "$p is missing 'publish = false' — workspace publish would try it"
    fi
  done
else
  info "(skipping community-file sanity per --skip-community)"
fi

# -----------------------------------------------------------------------------
# phase 3 — local gate
# -----------------------------------------------------------------------------

step "Phase 3: local gate (fmt + clippy + test + deny + release build)"
run cargo fmt --all -- --check
run cargo clippy --workspace --all-targets -- -D warnings
run cargo test --workspace --all-targets
run cargo deny check
run cargo build --release
ok "local gate green"

# -----------------------------------------------------------------------------
# phase 4 — dry-run publish for every crate
# -----------------------------------------------------------------------------

if [[ $SKIP_PUBLISH -eq 0 ]]; then
  if [[ $SKIP_DRY_RUN -eq 1 ]]; then
    info "(skipping Phase 4 dry-run per --skip-dry-run)"
  else
    step "Phase 4: cargo publish --dry-run for deck-core (path-dep free)"
    # NOTE: cargo publish --dry-run resolves path dependencies against
    # the crates.io index, not the workspace path. For any crate that
    # depends on another deck-* crate (everything except deck-core),
    # the dry-run *will* fail with "no matching package" until the
    # upstream is on crates.io. Phase 7 publishes in topological order
    # with index polling, so production publish is fine. We only
    # dry-run deck-core (which has no deck-* deps) as a sanity check;
    # pass --skip-dry-run to bypass entirely on a re-run.
    run cargo publish --dry-run --allow-dirty --manifest-path crates/deck-core/Cargo.toml
    ok "deck-core dry-run passed; rest will be verified live in Phase 7"
  fi
else
  info "(skipping crates.io dry-run per --skip-publish)"
fi

# -----------------------------------------------------------------------------
# phase 5 — final confirmation
# -----------------------------------------------------------------------------

step "Phase 5: confirm intent"
cat <<MSG
    tag             : $TAG  (on $(git rev-parse --short HEAD))
    crates.io       : $([[ $SKIP_PUBLISH -eq 0 ]] && echo "publish 8 lib crates + binary" || echo "SKIPPED")
    GitHub release  : create v$TARGET_VERSION with CHANGELOG body
    repo metadata   : idempotent re-assert (description, topics)

MSG

if [[ $EXECUTE -eq 0 ]]; then
  warn "this was a dry run. Re-invoke with --execute to actually release."
  exit 0
fi

read -r -p "    proceed? [y/N] " ans
case "$ans" in
  y|Y|yes|YES) ;;
  *) die "aborted by user" 2 ;;
esac

# -----------------------------------------------------------------------------
# phase 6 — tag and push
# -----------------------------------------------------------------------------

step "Phase 6: tag $TAG and push"

if git rev-parse "$TAG" >/dev/null 2>&1; then
  EXISTING="$(git rev-list -n 1 "$TAG")"
  if [[ "$EXISTING" == "$(git rev-parse HEAD)" ]]; then
    ok "$TAG already exists at HEAD"
  elif [[ $ALLOW_TAG_MISMATCH -eq 1 ]]; then
    warn "$TAG points at $EXISTING (HEAD is $(git rev-parse HEAD))."
    warn "Continuing because --allow-tag-mismatch was passed. crates.io will see"
    warn "the HEAD's Cargo.toml; ensure the version still matches the tag."
  else
    die "$TAG already exists but points at $EXISTING (HEAD is $(git rev-parse HEAD)). Refusing to move a public tag — bump version, or pass --allow-tag-mismatch if HEAD only differs in non-crate files (scripts/, docs/, etc.)."
  fi
else
  run git tag -a "$TAG" -m "ono-sendai $TAG"
fi

if git ls-remote --tags origin "$TAG" | grep -q "$TAG"; then
  ok "$TAG already on origin"
else
  run git push origin "$TAG"
fi

# -----------------------------------------------------------------------------
# phase 7 — cargo publish, topological, with index poll
# -----------------------------------------------------------------------------

if [[ $SKIP_PUBLISH -eq 0 ]]; then
  step "Phase 7: cargo publish (topological order, index-aware)"

  poll_index() {
    # poll_index <crate-name> <expected-version>
    local name="$1"; local want="$2"
    local tries=0
    while (( tries < 24 )); do
      if cargo search "$name" --limit 1 2>/dev/null \
         | grep -E "^${name} = \"${want}\"" >/dev/null; then
        return 0
      fi
      tries=$((tries+1))
      sleep 5
    done
    return 1
  }

  publish_with_retry() {
    # publish_with_retry <crate-name> <manifest-or-empty>
    # Retries on HTTP 429 (crates.io new-crate rate limit, default 1 per
    # 10 minutes) up to 6 times = ~70 minutes of backoff. All other
    # failures bubble up immediately.
    local cname="$1"; local manifest="$2"
    local attempt=0; local max=6; local sleep_secs=660
    local out rc=0
    while (( attempt < max )); do
      if [[ -n "$manifest" ]]; then
        out=$(cargo publish --manifest-path "$manifest" 2>&1) && rc=0 || rc=$?
      else
        out=$(cargo publish 2>&1) && rc=0 || rc=$?
      fi
      printf '%s\n' "$out"
      if (( rc == 0 )); then
        return 0
      fi
      if echo "$out" | grep -q "429 Too Many Requests"; then
        attempt=$((attempt+1))
        warn "rate-limited on $cname (attempt $attempt/$max). Sleeping $sleep_secs s before retry."
        sleep "$sleep_secs"
        continue
      fi
      return "$rc"
    done
    return 1
  }

  for crate_path in "${PUBLISH_ORDER[@]}"; do
    local_manifest="$crate_path/Cargo.toml"
    [[ "$crate_path" == "." ]] && local_manifest="Cargo.toml"
    crate_name="$(awk -F\" '/^\[package\]/{f=1} f && /^name[[:space:]]*=/{print $2; exit}' "$local_manifest")"
    [[ -n "$crate_name" ]] || die "could not parse crate name from $local_manifest"

    if cargo search "$crate_name" --limit 1 2>/dev/null \
       | grep -E "^${crate_name} = \"${TARGET_VERSION}\"" >/dev/null; then
      ok "$crate_name $TARGET_VERSION already on crates.io — skipping"
      continue
    fi

    info "publishing $crate_name $TARGET_VERSION"
    if [[ "$crate_path" == "." ]]; then
      if [[ $EXECUTE -eq 1 ]]; then
        publish_with_retry "$crate_name" "" \
          || die "$crate_name publish failed after retries. Re-run to resume." 3
      else
        info "(dry-run) would publish $crate_name"
      fi
    else
      if [[ $EXECUTE -eq 1 ]]; then
        publish_with_retry "$crate_name" "$local_manifest" \
          || die "$crate_name publish failed after retries. Re-run to resume." 3
      else
        info "(dry-run) would publish $crate_name"
      fi
    fi

    if [[ $EXECUTE -eq 1 ]]; then
      info "waiting for crates.io index to surface $crate_name=$TARGET_VERSION (max 2m)"
      if poll_index "$crate_name" "$TARGET_VERSION"; then
        ok "$crate_name visible on index"
      else
        die "$crate_name not visible on crates.io index after 2 minutes. Re-run the script to resume." 3
      fi
    fi
  done
fi

# -----------------------------------------------------------------------------
# phase 8 — GitHub release
# -----------------------------------------------------------------------------

step "Phase 8: GitHub release for $TAG"

if gh release view "$TAG" --repo "$REPO_SLUG" >/dev/null 2>&1; then
  ok "release $TAG already exists on GitHub"
else
  # Extract the section for $TARGET_VERSION from CHANGELOG.md.
  NOTES="$(awk -v ver="$TARGET_VERSION" '
    $0 ~ "^## \\[" ver "\\]" {flag=1; next}
    flag && /^## \[/ {flag=0}
    flag {print}
  ' CHANGELOG.md)"

  if [[ -z "$NOTES" ]]; then
    warn "could not extract a CHANGELOG section for $TARGET_VERSION — falling back to generic body"
    NOTES="See CHANGELOG.md for $TAG."
  fi

  TMP_NOTES="$(mktemp)"
  printf "%s\n\n---\nGenerated by scripts/release.sh\n" "$NOTES" > "$TMP_NOTES"
  if [[ $EXECUTE -eq 1 ]]; then
    gh release create "$TAG" --repo "$REPO_SLUG" --title "ono-sendai $TAG" --notes-file "$TMP_NOTES"
  else
    info "(dry-run) would: gh release create $TAG --notes-file <generated>"
  fi
  rm -f "$TMP_NOTES"
fi

# -----------------------------------------------------------------------------
# phase 9 — repo metadata (idempotent re-assert)
# -----------------------------------------------------------------------------

step "Phase 9: repo metadata (idempotent)"

run gh repo edit "$REPO_SLUG" \
  --description "Console Cowboy deck — pre-alpha Rust workspace for an offline-first terminal TUI agent (ratatui + local LLM + MCP host). Sandbox enforcement lands in 0.2. MIT." \
  --homepage "https://crates.io/crates/ono-sendai" \
  --add-topic rust --add-topic tui --add-topic llm --add-topic mcp \
  --add-topic ollama --add-topic agent --add-topic cyberdeck \
  --add-topic ratatui --add-topic neuromancer --add-topic offline-first \
  --add-topic sandbox --add-topic seccomp \
  --enable-issues --enable-discussions=true --enable-wiki=false \
  --enable-projects=false --delete-branch-on-merge

# -----------------------------------------------------------------------------
# phase 10 — print remaining manual TODOs
# -----------------------------------------------------------------------------

step "Phase 10: remaining manual TODOs"
cat <<TODO

    The release is complete. The following items remain genuinely manual
    (they require external accounts, judgement calls, or one-time secrets
    the script declines to handle):

    1. PUBLISH AWARENESS POSTS (optional, marketing)
       - https://www.reddit.com/r/rust/ "Show /r/rust"
       - https://this-week-in-rust.org/ — submit PR with mention
       - Mastodon / Bluesky / X if you maintain accounts

    2. DISTRIBUTION (optional, can defer to 0.2)
       - Homebrew tap: create hinanohart/homebrew-tap repo, add Formula
       - AUR: aur.archlinux.org account, ono-sendai-bin PKGBUILD
       - cargo-binstall metadata: already works via crates.io, no action

    3. SOCIAL PREVIEW IMAGE (optional, SEO)
       - GitHub Settings -> General -> Social preview image
       - 1280x640 PNG/JPG, no script API for it

    4. BRANCH PROTECTION (optional, OSS 0.1 typically skips this)
       - gh api repos/${REPO_SLUG}/branches/main/protection --method PUT \\
           --input <(echo '{"required_status_checks":{"strict":true,"contexts":["CI / clippy","CI / test (ubuntu-latest)"]},"enforce_admins":false,"required_pull_request_reviews":null,"restrictions":null,"allow_force_pushes":false,"allow_deletions":false}')

    5. CODEOWNERS (optional)
       - If you accept PRs, create .github/CODEOWNERS with @hinanohart on all

    6. CRATES.IO METADATA POLISH (defer to next bump)
       - per-crate keywords/categories (workspace.package fields only
         apply to the binary; lib crates inherit nothing for those two
         specific fields by Cargo design)

    7. DEPENDABOT FINDING
       - GitHub flagged 1 LOW vulnerability on the lru crate. Review at:
         https://github.com/${REPO_SLUG}/security/dependabot
       - Likely resolved automatically by the cargo dependabot updates
         you'll receive weekly under .github/dependabot.yml.

TODO

ok "all automated phases complete. See list above for what is left."
