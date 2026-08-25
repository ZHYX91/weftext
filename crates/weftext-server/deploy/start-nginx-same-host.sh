#!/bin/sh
set -eu

: "${WEFTEXT_PROXY_SECRET_FILE:?set WEFTEXT_PROXY_SECRET_FILE}"
: "${WEFTEXT_PUBLIC_ORIGIN:?set WEFTEXT_PUBLIC_ORIGIN}"
: "${WEFTEXT_PUBLIC_AUTHORITY:?set WEFTEXT_PUBLIC_AUTHORITY}"
: "${WEFTEXT_PUBLIC_HOST:?set WEFTEXT_PUBLIC_HOST}"
: "${WEFTEXT_HTTPS_PORT:?set WEFTEXT_HTTPS_PORT}"
: "${WEFTEXT_TLS_CERTIFICATE:?set WEFTEXT_TLS_CERTIFICATE}"
: "${WEFTEXT_TLS_PRIVATE_KEY:?set WEFTEXT_TLS_PRIVATE_KEY}"

case "${WEFTEXT_PUBLIC_AUTHORITY}" in
    *[!a-z0-9.:-]*|'')
        echo "WEFTEXT_PUBLIC_AUTHORITY is not a safe lowercase authority" >&2
        exit 64
        ;;
esac
case "${WEFTEXT_PUBLIC_HOST}" in
    *[!a-z0-9.-]*|'')
        echo "WEFTEXT_PUBLIC_HOST is not a safe lowercase host" >&2
        exit 64
        ;;
esac
case "${WEFTEXT_HTTPS_PORT}" in
    *[!0-9]*|''|0|0*)
        echo "WEFTEXT_HTTPS_PORT is invalid" >&2
        exit 64
        ;;
esac
if [ "${#WEFTEXT_HTTPS_PORT}" -gt 5 ] || [ "${WEFTEXT_HTTPS_PORT}" -gt 65535 ]; then
    echo "WEFTEXT_HTTPS_PORT is invalid" >&2
    exit 64
fi
if [ "${WEFTEXT_HTTPS_PORT}" = "443" ]; then
    expected_authority=${WEFTEXT_PUBLIC_HOST}
else
    expected_authority=${WEFTEXT_PUBLIC_HOST}:${WEFTEXT_HTTPS_PORT}
fi
if [ "${WEFTEXT_PUBLIC_AUTHORITY}" != "${expected_authority}" ]; then
    echo "WEFTEXT_PUBLIC_AUTHORITY does not match WEFTEXT_PUBLIC_HOST and WEFTEXT_HTTPS_PORT" >&2
    exit 64
fi
if [ "${WEFTEXT_PUBLIC_ORIGIN}" != "https://${expected_authority}" ]; then
    echo "WEFTEXT_PUBLIC_ORIGIN does not match the canonical HTTPS authority" >&2
    exit 64
fi
for tls_path in "${WEFTEXT_TLS_CERTIFICATE}" "${WEFTEXT_TLS_PRIVATE_KEY}"; do
    case "${tls_path}" in
        /*) ;;
        *)
            echo "TLS paths must be absolute" >&2
            exit 64
            ;;
    esac
    case "${tls_path}" in
        *[!A-Za-z0-9_./:-]*)
            echo "TLS path contains unsupported characters" >&2
            exit 64
            ;;
    esac
    if [ ! -f "${tls_path}" ] || [ -L "${tls_path}" ] || [ ! -r "${tls_path}" ]; then
        echo "TLS file is unavailable or linked" >&2
        exit 66
    fi
done
private_key_mode=$(stat -c '%a' "${WEFTEXT_TLS_PRIVATE_KEY}")
case "${private_key_mode}" in
    *00) ;;
    *)
        echo "TLS private key must not be accessible by group or others" >&2
        exit 77
        ;;
esac

case "${WEFTEXT_PROXY_SECRET_FILE}" in
    /*) ;;
    *)
        echo "reverse-proxy-secret path must be absolute" >&2
        exit 64
        ;;
esac
secret_file=${WEFTEXT_PROXY_SECRET_FILE}
if [ ! -f "${secret_file}" ] || [ -L "${secret_file}" ] || [ ! -r "${secret_file}" ]; then
    echo "protected reverse-proxy-secret is unavailable" >&2
    exit 69
fi
secret_mode=$(stat -c '%a' "${secret_file}")
case "${secret_mode}" in
    *00) ;;
    *)
        echo "reverse-proxy-secret must not be accessible by group or others" >&2
        exit 77
        ;;
esac
IFS= read -r WEFTEXT_PROXY_TOKEN < "${secret_file}"
case "${WEFTEXT_PROXY_TOKEN}" in
    *[!0-9a-f]*|'')
        echo "reverse-proxy-secret is invalid" >&2
        exit 65
        ;;
esac
if [ "${#WEFTEXT_PROXY_TOKEN}" -ne 64 ]; then
    echo "reverse-proxy-secret is invalid" >&2
    exit 65
fi
export WEFTEXT_PROXY_TOKEN

template="${WEFTEXT_NGINX_TEMPLATE:-/etc/nginx/weftext-same-host.conf.template}"
generated="${WEFTEXT_NGINX_CONFIG:-/run/weftext-nginx.conf}"
if [ ! -f "${template}" ] || [ -L "${template}" ] || [ ! -r "${template}" ]; then
    echo "audited nginx template is unavailable or linked" >&2
    exit 66
fi
mkdir -p \
    /tmp/weftext-nginx/client-body \
    /tmp/weftext-nginx/proxy
umask 077
temporary="${generated}.tmp.$$"
trap 'rm -f "${temporary}"' EXIT HUP INT TERM
envsubst '${WEFTEXT_PUBLIC_HOST} ${WEFTEXT_PUBLIC_AUTHORITY} ${WEFTEXT_HTTPS_PORT} ${WEFTEXT_TLS_CERTIFICATE} ${WEFTEXT_TLS_PRIVATE_KEY} ${WEFTEXT_PROXY_TOKEN}' \
    < "${template}" > "${temporary}"
nginx -t -q -c "${temporary}"
mv "${temporary}" "${generated}"
trap - EXIT HUP INT TERM
unset WEFTEXT_PROXY_TOKEN
exec nginx -c "${generated}" -g 'daemon off;'
