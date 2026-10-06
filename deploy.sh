#!/usr/bin/env bash
# Builds the player image for the cluster, pushes it to Harbor, points the
# devops repo's Fleet bundle at it, and waits for the new pod.
#
#   ./deploy.sh
#
# DEVOPS_REPO names the devops checkout (default ~/workspace/devops).
set -euo pipefail

cd "$(dirname "$0")"

IMAGE=harbor.home.gregdonald.com/player/player
DEVOPS_REPO="${DEVOPS_REPO:-$HOME/workspace/devops}"
DEPLOYMENT=fleet/player/deployment.yaml

if ! git diff --quiet HEAD; then
  echo "player has uncommitted changes, commit them first so the image tag names what it was built from" >&2
  exit 1
fi

if ! git -C "$DEVOPS_REPO" diff --quiet HEAD -- "$DEPLOYMENT"; then
  echo "$DEVOPS_REPO/$DEPLOYMENT has uncommitted changes" >&2
  exit 1
fi

version=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/server/Cargo.toml)
tag="$version-$(git rev-parse --short HEAD)"

echo "== building $IMAGE:$tag for linux/amd64"
docker buildx build --platform linux/amd64 --tag "$IMAGE:$tag" --push .

echo "== pointing $DEPLOYMENT at $tag"
sed -i '' "s#image: $IMAGE:.*#image: $IMAGE:$tag#" "$DEVOPS_REPO/$DEPLOYMENT"
grep -n "image: $IMAGE:$tag" "$DEVOPS_REPO/$DEPLOYMENT"

git -C "$DEVOPS_REPO" commit --quiet -m "Deploy player $tag" -- "$DEPLOYMENT"
git -C "$DEVOPS_REPO" push --quiet

echo "== waiting for Fleet to apply $tag"
until [ "$(kubectl -n player get deploy player -o jsonpath='{.spec.template.spec.containers[0].image}')" = "$IMAGE:$tag" ]; do
  sleep 5
done

kubectl -n player rollout status deploy/player --timeout=300s

echo "== deployed $IMAGE:$tag"
