#!/bin/bash
set -e
clear

bash ~/environment/busy.sh

# =======================
#   CONFIG & REGISTRY
# =======================
REGISTRY="astepan0v.registry.twcstorage.ru"
SERVICE_NAME="adatari-ip-service"
LOCAL_IMAGE="adatari/ip"

echo "Logging into registry $REGISTRY..."
docker login $REGISTRY

TS=$(date +"%Y%m%d-%H%M%S")
echo "Tag = $TS"

REMOTE_IMAGE="$REGISTRY/$LOCAL_IMAGE:$TS"
REMOTE_LATEST="$REGISTRY/$LOCAL_IMAGE:latest"

# =======================
#   BUILD
# =======================
echo "================ Build: $SERVICE_NAME ================"
docker build -t "$LOCAL_IMAGE" .

# =======================
#   TAG
# =======================
echo "Tagging:"
echo "  $LOCAL_IMAGE -> $REMOTE_IMAGE"
echo "  $LOCAL_IMAGE -> $REMOTE_LATEST"

docker tag "$LOCAL_IMAGE" "$REMOTE_IMAGE"
docker tag "$LOCAL_IMAGE" "$REMOTE_LATEST"

# =======================
#   PUSH
# =======================
echo "Pushing $REMOTE_IMAGE"
docker push "$REMOTE_IMAGE"

echo "Pushing $REMOTE_LATEST"
docker push "$REMOTE_LATEST"

# =======================
#   DEPLOY.ENV GENERATION
# =======================
echo "IMAGE_TAG=$TS" > deploy.env
echo "REGISTRY=$REGISTRY" >> deploy.env
echo "IMAGE_NAME=$LOCAL_IMAGE" >> deploy.env

echo "DONE "
echo "Generated deploy.env:"
cat deploy.env

# =======================
#   CLEANUP
# =======================
bash ~/environment/free.sh

echo "=============== DONE ==============="
echo "Image pushed:"
echo "  $REMOTE_IMAGE"
echo "Use on server:"
echo "  docker pull $REMOTE_IMAGE"
echo "  docker run -p 8080:8080 $REMOTE_IMAGE"
