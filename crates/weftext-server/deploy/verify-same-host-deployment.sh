#!/bin/sh
set -eu

origin=${1:?usage: verify-same-host-deployment.sh https://authority}
case "${origin}" in
    https://*) ;;
    *)
        echo "verification origin must use https" >&2
        exit 64
        ;;
esac

live=$(curl --fail --silent --show-error --proto '=https' --tlsv1.2 "${origin}/api/v1/health/live")
ready=$(curl --fail --silent --show-error --proto '=https' --tlsv1.2 "${origin}/api/v1/health/ready")
printf '%s\n' "${live}" | grep -q '"status":"ok"'
printf '%s\n' "${ready}" | grep -q '"status":"ready"'
printf '%s\n' "same-host Weftext deployment is live and ready at ${origin}"
