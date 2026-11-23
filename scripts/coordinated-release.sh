#!/usr/bin/env bash
# GitHub/GHCR transport for marreq-release. Validation and asset schemas live in Rust.
set -euo pipefail
umask 077

mode=${1:?plan or publish required}
work=${RELEASE_WORK:?release workspace required}
head=${RELEASE_HEAD:?validated source required}
tag=${GITHUB_REF_NAME:?product tag required}
repo=${GITHUB_REPOSITORY:?repository required}
release=${RELEASE_CLI:-target/debug/marreq-release}
contracts=${CONTRACT_CLI:-target/debug/marreq-contracts}
plan=$work/plan.json
assets=$work/assets
components=(server cloud frontend mcp)
files=(release-manifest.json {server,cloud}-provenance.json {server,cloud}-compatibility.json {server,cloud,frontend,mcp}-contract.json)

fail() { echo "Release: $*" >&2; exit 1; }
api() { gh api "repos/$repo/$1" "${@:2}"; }
cli() {
  local output=$1
  shift
  "$release" --format json "$@" > "$output" || { cat "$output" >&2; return 1; }
}
asset_record() {
  jq -ce --arg file "$2" '[(.assets // [])[] | select(.name == $file)] |
    if length > 1 then error("duplicate release asset") else .[0] // {} end' <<< "$1"
}
download() {
  local row=$1 output=$2 id expected
  id=$(jq -er 'select(.state == "uploaded") | .id' <<< "$row") || return 1
  expected=$(jq -er '.digest | select(test("^sha256:[a-f0-9]{64}$"))' <<< "$row") || return 1
  api "releases/assets/$id" -H 'Accept: application/octet-stream' > "$output" || return 1
  echo "${expected#sha256:}  $output" | sha256sum --check --status
}
upload() {
  local id=$1 file=$2
  curl --fail-with-body --silent --show-error --request POST \
    --header "Authorization: Bearer $GH_TOKEN" --header 'Content-Type: application/json' \
    --data-binary "@$assets/$file" \
    "https://uploads.github.com/repos/$repo/releases/$id/assets?name=$file" >/dev/null
}
entries() {
  jq -cer '.selection | to_entries[]' "$plan"
}
assert_origin() {
  local allow_advance=$1 branch tip ref
  branch=$(api branches/main)
  tip=$(jq -er 'select(.protected == true) | .commit.sha | select(test("^[a-f0-9]{40}$"))' <<< "$branch") ||
    fail "main protection or identity could not be verified"
  if [[ "$tip" != "$head" ]]; then
    [[ "$allow_advance" == true ]] || fail "candidate is no longer the tip of protected main"
    git fetch --no-tags --no-write-fetch-head origin "$tip"
    git log --first-parent --format=%H "$tip" | awk -v head="$head" '$0 == head {found=1} END {exit !found}' ||
      fail "journal candidate is no longer on the first-parent history of protected main"
  fi
  ref=$(api "git/ref/tags/$tag")
  jq -e --arg oid "$TAG_OBJECT" '.object.type == "tag" and .object.sha == $oid' <<< "$ref" >/dev/null ||
    fail "candidate annotation changed"
}

assert_immutable_policy() {
  [[ -n ${MARREQ_RELEASE_POLICY_TOKEN:-} ]] || fail "MARREQ_RELEASE_POLICY_TOKEN is required"
  GH_TOKEN=$MARREQ_RELEASE_POLICY_TOKEN api immutable-releases -H 'X-GitHub-Api-Version: 2026-03-10' |
    jq -e '.enabled == true' >/dev/null || fail "enable immutable releases on the repository"
}

prepare() {
  local previous prior_tag prior entry component action version file expected variant
  local -a flags bundle_args
  mkdir -p "$assets" "$work/prior"
  cli "$work/base.json" release-base --head "$head" --tag "$tag"
  previous=$(jq -er .previous "$work/base.json")
  prior_tag=$(jq -r '.tag // ""' "$work/base.json")
  flags=(--publication)
  if [[ -z "$prior_tag" ]]; then
    [[ ${MARREQ_BOOTSTRAP_TAG:-} == "$tag" ]] ||
      fail "set MARREQ_BOOTSTRAP_TAG to $tag to authorize the first coordinated publication"
    flags+=(--bootstrap-publication)
  else
    prior=$(api "releases/tags/$prior_tag")
    jq -e '.draft == false and .prerelease == false and .immutable == true and .author.login == "github-actions[bot]"' \
      <<< "$prior" >/dev/null || fail "$prior_tag is not a trusted immutable release"
    download "$(asset_record "$prior" release-manifest.json)" "$work/prior/release-manifest.json" ||
      fail "$prior_tag is missing its product manifest"
    jq -e --arg tag "$prior_tag" --arg head "$previous" '
      .schema_version == 1 and .product.tag == $tag and .product.source_sha == $head and
      .workflow.path == ".github/workflows/release-artifacts.yml"
    ' "$work/prior/release-manifest.json" >/dev/null || fail "$prior_tag product manifest has unexpected provenance"
    for file in {server,cloud}-provenance.json {server,cloud,frontend,mcp}-contract.json; do
      download "$(asset_record "$prior" "$file")" "$work/prior/$file" ||
        fail "missing or corrupt prior $file on $prior_tag"
    done
    for variant in server cloud; do
      file="$variant-provenance.json"
      expected=$(jq -er --arg variant "$variant" --arg file "$file" '
        .validation.deployment_provenance[$variant] | select(.file == $file) |
        .sha256 | select(test("^[a-f0-9]{64}$"))
      ' "$work/prior/release-manifest.json")
      echo "$expected  $work/prior/$file" | sha256sum --check --status ||
        fail "$prior_tag product manifest disagrees with $file"
    done
    flags+=(--server-provenance "$work/prior/server-provenance.json" --cloud-provenance "$work/prior/cloud-provenance.json")
  fi
  cli "$plan" release --head "$head" --tag "$tag" --previous "$previous" "${flags[@]}"
  for component in "${components[@]}"; do
    entry=$(jq -ec --arg key "$component" '.selection[$key]' "$plan")
    action=$(jq -er .action <<< "$entry")
    version=$(jq -er .version <<< "$entry")
    if [[ "$action" == reuse ]]; then
      cp "$work/prior/$component-contract.json" "$assets/"
    else
      bundle_args=(--component "$component")
      if [[ "$component" == server || "$component" == cloud ]]; then
        bundle_args=(--component backend --variant "$component")
      fi
      "$contracts" bundle "${bundle_args[@]}" --version "$version" --output "$assets/$component-contract.json"
    fi
  done
  "$release" release-assets --plan "$plan" --directory "$assets"
  for variant in server cloud; do
    "$contracts" verify --manifest "$assets/$variant-compatibility.json"
  done
  jq -er .notes "$plan" > "$work/notes.md"
}

# Query the actual registry API. Only an explicit registry "unknown" response
# means a slot is absent. Auth, transport and unexpected responses abort.
registry_digest() {
  local image=$1 ref=$2 scope token status
  local headers body
  local accept='application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json'
  accept+=', application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json'
  scope="repository:${image#ghcr.io/}:pull,push"
  token=$(curl --silent --show-error --fail --connect-timeout 15 --max-time 120 \
    --user "$GITHUB_ACTOR:$GH_TOKEN" --get --data-urlencode "scope=$scope" \
    --data-urlencode service=ghcr.io https://ghcr.io/token |
    jq -er '(.token // .access_token) | select(type == "string" and length > 0)') || return 1
  headers=$(mktemp "$work/headers.XXXXXX")
  body=$(mktemp "$work/body.XXXXXX")
  status=$(curl --silent --show-error --connect-timeout 15 --max-time 120 \
    --header "Authorization: Bearer $token" --header "Accept: $accept" \
    --dump-header "$headers" --output "$body" --write-out '%{http_code}' \
    "https://ghcr.io/v2/${image#ghcr.io/}/manifests/$ref") || return 1
  local digest hash
  case "$status" in
    200)
      digest=$(awk 'tolower($1) == "docker-content-digest:" {gsub(/\r/, "", $2); print $2}' "$headers")
      hash=$(sha256sum "$body")
      [[ "$digest" == "sha256:${hash%% *}" ]] || { echo "Invalid registry digest for $image:$ref" >&2; return 1; }
      echo "$digest"
      ;;
    404)
      jq -e '.errors | length > 0 and all(.[]; .code == "MANIFEST_UNKNOWN" or .code == "NAME_UNKNOWN")' "$body" >/dev/null ||
        { echo "Unexpected registry 404 for $image:$ref" >&2; return 1; }
      ;;
    *) echo "Registry returned $status for $image:$ref" >&2; return 1 ;;
  esac
  rm -f "$headers" "$body"
}

load_entry() {
  component=$(jq -er .key <<< "$1")
  image=$(jq -er .value.image <<< "$1")
  version=$(jq -er .value.version <<< "$1")
  source=$(jq -er .value.source_sha <<< "$1")
  source_tag=$(jq -er .value.source_tag <<< "$1")
  action=$(jq -er .value.action <<< "$1")
  contract_hash=$(sha256sum "$assets/$component-contract.json")
  contract_hash=${contract_hash%% *}
}
verify_image() {
  local digest=$1 observed
  observed=$(registry_digest "$image" "$digest")
  [[ "$observed" == "$digest" ]] || fail "$image@$digest is unavailable"
  docker pull --quiet --platform linux/amd64 "$image@$digest"
  docker image inspect "$image@$digest" |
    jq -e --arg title "${image##*/}" --arg version "$version" --arg source "$source" --arg contract "$contract_hash" '
      .[0] | .Architecture == "amd64" and .Os == "linux" and
      (.Config.Labels | .["org.opencontainers.image.title"] == $title and
        .["org.opencontainers.image.source"] == "https://github.com/MarreqSW/Marreq" and
        .["org.opencontainers.image.version"] == $version and
        .["org.opencontainers.image.revision"] == $source and
        .["org.marreq.contract.sha256"] == $contract)' >/dev/null ||
    fail "$component OCI identity differs from the release plan"
}
remote_source() {
  local result ref
  # ls-remote succeeds with empty output for a genuinely absent tag.
  result=$(git ls-remote origin "refs/tags/$source_tag" "refs/tags/$source_tag^{}") || return 1
  ref=$(awk '/\^\{\}$/ {print $1}' <<< "$result")
  if [[ -z "$ref" ]]; then ref=$(awk '{print $1}' <<< "$result"); fi
  printf '%s' "$ref"
}
check_slot() {
  local expected=$1 observed remote
  observed=$(registry_digest "$image" "$version")
  remote=$(remote_source)
  if [[ "$action" == reuse ]]; then
    [[ -n "$observed" ]] || fail "$image:$version is unavailable"
    [[ "$observed" == "$expected" && "$remote" == "$source" ]] ||
      fail "$component retained version digest or source tag changed"
  else
    [[ -z "$observed" || "$observed" == "$expected" ]] || fail "$image:$version is already occupied"
    [[ -z "$remote" || ( -n "$expected" && "$remote" == "$source" ) ]] ||
      fail "$source_tag is already occupied"
  fi
}

stage() {
  local staging="$image:staging-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}" file digest
  local -a args=()
  case "$component" in
    server|cloud) file=docker/Dockerfile; args=(--build-arg "MARREQ_BIN=marreq-$component");;
    frontend) file=docker/frontend/Dockerfile;;
    mcp) file=mcp-server/Dockerfile;;
    *) fail "unexpected component $component";;
  esac
  # Keep BuildKit cache across components in this release. Server and Cloud share
  # most Rust layers; pruning between builds forces repeated dependency compiles.
  docker buildx build --file "$work/context/$file" --platform linux/amd64 --push \
    --tag "$staging" --metadata-file "$work/$component-build.json" \
    --build-arg "MARREQ_VERSION=$version" --build-arg "MARREQ_GIT_SHA=$source" \
    --label "org.marreq.contract.sha256=$contract_hash" "${args[@]}" "$work/context" >&2 ||
    fail "$component build or staging push failed"
  digest=$(jq -er '."containerimage.digest" | select(test("^sha256:[a-f0-9]{64}$"))' "$work/$component-build.json") ||
    fail "Buildx did not report an immutable digest for $component"
  [[ $(registry_digest "$image" "${staging##*:}") == "$digest" ]] || fail "staged $component digest differs from the build result"
  verify_image "$digest" >&2
  if [[ "$component" == server || "$component" == cloud ]]; then
    scripts/test-migration-image.sh "$image@$digest" "$component" "$version" "$source" >&2
  else
    python3 scripts/test-artifact-image.py "$component" "$image@$digest" "$version" "$source" >&2
  fi
  echo "$digest"
}

publish() {
  local draft listing id journal timestamp digests entry digest observed current manifest file
  local resumed=false
  local draft_name="Marreq $tag (run $GITHUB_RUN_ID)"
  local component image version source source_tag action contract_hash
  local -a rows
  mkdir -p "$work/remote"
  assert_immutable_policy
  listing=$(gh api --paginate --slurp "repos/$repo/releases?per_page=100" |
    jq -ce --arg tag "$tag" '[.[][] | select(.tag_name == $tag)] |
      if length > 1 then error("duplicate releases for tag") else .[0] // {} end')
  draft=false
  id=$(jq -r '.id // ""' <<< "$listing")
  if [[ -n "$id" ]]; then
    jq -e --arg draft "$draft_name" '
      .author.login == "github-actions[bot]" and .prerelease == false and
      (if .draft then .name == $draft else .immutable == true end)' <<< "$listing" >/dev/null ||
      fail "an unexpected GitHub Release already owns $tag"
    draft=$(jq -er '.draft | tostring' <<< "$listing")
  fi
  journal=$(asset_record "$listing" release-manifest.json)
  digests=$work/digests.json
  manifest=$assets/release-manifest.json
  mapfile -t rows < <(entries)
  [[ ${#rows[@]} == 4 ]] || fail "expected four selected components"
  if [[ $(jq -r '.state // ""' <<< "$journal") == uploaded ]]; then
    download "$journal" "$work/remote/release-manifest.json" || fail "publication journal is corrupt"
    jq -e --argjson run "$GITHUB_RUN_ID" --arg head "$head" --arg tag "$tag" --arg oid "$TAG_OBJECT" '
      .workflow.run_id == $run and .product.source_sha == $head and .product.tag == $tag and .product.tag_object == $oid
    ' "$work/remote/release-manifest.json" >/dev/null || fail "publication journal belongs to a different run or candidate"
    timestamp=$(jq -er .recorded_at_unix "$work/remote/release-manifest.json")
    jq -e '.components | map_values(.digest)' "$work/remote/release-manifest.json" > "$digests"
    resumed=true
    assert_origin "$resumed"
  else
    [[ -z "$id" || "$draft" == true ]] || fail "published release is missing its journal"
    assert_origin "$resumed"
    # Check every slot before staging any new image.
    for entry in "${rows[@]}"; do
      load_entry "$entry"
      digest=$(jq -r '.value.retained_digest // ""' <<< "$entry")
      if [[ "$component" == cloud && $(jq -er '.bootstrap | tostring' "$plan") == true ]]; then
        # A repository that does not yet exist may refuse a read token.
        # The staging push initializes it. Version slots are checked below
        # before writing the journal or assigning any canonical alias.
        observed=$(remote_source)
        [[ -z "$observed" ]] || fail "$source_tag is already occupied"
      else
        check_slot "$digest"
      fi
      if [[ "$action" == reuse ]]; then verify_image "$digest"; fi
    done
    # The publisher keeps umask 077 for credentials, journals and evidence.
    # GNU tar applies that umask by default for non-root extractors, which can
    # strip group/other read bits from archived files. Preserve archive modes so
    # Docker COPY leaves scripts such as docker-entrypoint.sh readable to the
    # non-root runtime user after chmod +x.
    mkdir -p "$work/context"
    git archive "$head" | tar --same-permissions -x -C "$work/context"
    local -a pairs=()
    for entry in "${rows[@]}"; do
      load_entry "$entry"
      if [[ "$action" == build ]]; then
        # Do not run stage in a command substitution with errexit disabled.
        stage > "$work/$component.digest"
        digest=$(cat "$work/$component.digest")
      else
        digest=$(jq -er .value.retained_digest <<< "$entry")
      fi
      pairs+=("$component" "$digest")
    done
    jq -n --args '$ARGS.positional as $a | reduce range(0; ($a|length); 2) as $i ({}; .[$a[$i]] = $a[$i+1])' "${pairs[@]}" > "$digests"
    timestamp=$(date -u +%s)
  fi

  "$release" release-assets --plan "$plan" --directory "$assets" --digests "$digests" \
    --run-id "$GITHUB_RUN_ID" --timestamp "$timestamp" --tag-object "$TAG_OBJECT"
  if [[ -f "$work/remote/release-manifest.json" ]]; then
    cmp "$manifest" "$work/remote/release-manifest.json" || fail "publication journal disagrees with the validated plan"
  fi
  # Check all digest availability, labels and version slots before any promotion.
  for entry in "${rows[@]}"; do
    load_entry "$entry"
    digest=$(jq -er --arg component "$component" '.[$component]' "$digests")
    if [[ "$action" == build && "$resumed" == false ]]; then
      check_slot ""
    else
      check_slot "$digest"
    fi
    verify_image "$digest"
  done

  assert_origin "$resumed"
  if [[ -z "$id" ]]; then
    listing=$(api releases -f tag_name="$tag" -f target_commitish="$head" -f name="$draft_name" -F draft=true -F prerelease=false)
    id=$(jq -er .id <<< "$listing")
    draft=true
  fi
  for file in "${files[@]}"; do
    current=$(asset_record "$listing" "$file")
    case "$(jq -r '.state // ""' <<< "$current")" in
      uploaded)
        download "$current" "$work/remote/$file" || fail "could not verify existing release asset: $file"
        cmp "$assets/$file" "$work/remote/$file" ||
          fail "existing release asset differs: $file"
        ;;
      ''|starter)
        [[ "$draft" == true ]] || fail "missing $file on published release"
        if [[ "$current" != '{}' ]]; then
          api "releases/assets/$(jq -er .id <<< "$current")" --method DELETE
        fi
        # The asset endpoint rejects duplicate names. Treat uploaded bytes as frozen.
        upload "$id" "$file" || fail "failed to retain $file in the draft release"
        ;;
      *) fail "unexpected asset state for $file";;
    esac
  done
  listing=$(api "releases/$id")
  for file in "${files[@]}"; do
    current=$(asset_record "$listing" "$file")
    observed=$(sha256sum "$assets/$file")
    jq -e --arg digest "sha256:${observed%% *}" '.state == "uploaded" and .digest == $digest' <<< "$current" >/dev/null ||
      fail "GitHub did not retain the expected bytes for $file"
  done

  # Only the recorded digests can be promoted. Matching aliases are a no-op.
  # GHCR has no conditional tag creation. Restrict all package writers to this
  # workflow and serialize the entire workflow at repository scope.
  for entry in "${rows[@]}"; do
    load_entry "$entry"
    digest=$(jq -er --arg component "$component" '.[$component]' "$digests")
    observed=$(registry_digest "$image" "$version")
    if [[ "$draft" == true && "$action" == build && -z "$observed" ]]; then
      docker buildx imagetools create --prefer-index=false --tag "$image:$version" "$image@$digest" ||
        fail "failed to assign the version alias $image:$version"
      observed=$(registry_digest "$image" "$version")
    fi
    [[ "$observed" == "$digest" ]] || fail "$image:$version does not resolve to $digest"
  done
  for entry in "${rows[@]}"; do
    load_entry "$entry"
    observed=$(remote_source)
    if [[ "$draft" == true && "$action" == build && -z "$observed" ]]; then
      GH_TOKEN=${MARREQ_SOURCE_TAG_TOKEN:-$GH_TOKEN} api git/refs \
        -f "ref=refs/tags/$source_tag" -f "sha=$source" >/dev/null ||
        fail "failed to create source tag $source_tag"
      observed=$(remote_source)
    fi
    [[ "$observed" == "$source" ]] || fail "$source_tag does not resolve to $source"
  done
  assert_origin true
  if [[ "$draft" == true ]]; then
    assert_immutable_policy
    api "releases/$id" --method PATCH -F draft=false -f name="Marreq $tag" -F "body=@$work/notes.md" |
      jq -e '.draft == false and .immutable == true' >/dev/null ||
      fail "GitHub did not report an immutable release; check repository settings"
  fi
  echo "Verified Marreq $tag release (GitHub release $id)"
}

case "$mode" in
  plan) prepare ;;
  publish) publish ;;
  *) fail "unknown operation $mode" ;;
esac
