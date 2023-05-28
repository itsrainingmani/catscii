_default:
  just --list

deploy:
  DOCKER_BUILDKIT=1 docker build \
    --ssh default \
    --secret id=shipyard-token,src=secrets/shipyard-token \
    --target app \
    --tag catscii \
    .
  fly deploy --local-only