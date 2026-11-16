#!/usr/bin/env bash
# Real HTTP/HTTPS ACME routing checks in isolated containers; no live DNS or cert issuance.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d /tmp/puzzgen-nginx.XXXXXXXX)"
name="puzzgen-nginx-check-${GITHUB_RUN_ID:-$$}"
cleanup() {
    docker rm -f "$name-bootstrap" "$name-tls" >/dev/null 2>&1 || true
    rm -rf -- "$fixture"
}
trap cleanup EXIT
host=puzzlegen.davewagner.org
mkdir -p "$fixture/webroot/.well-known/acme-challenge" "$fixture/cert"
printf 'puzzgen-acme-check' > "$fixture/webroot/.well-known/acme-challenge/check"
openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj "/CN=$host" \
    -addext "subjectAltName=DNS:$host" -keyout "$fixture/cert/privkey.pem" -out "$fixture/cert/fullchain.pem" 2>/dev/null
docker run -d --name "$name-bootstrap" -p 127.0.0.1::80 \
    -v "$root/deploy/nginx-bootstrap.conf:/etc/nginx/conf.d/default.conf:ro" \
    -v "$fixture/webroot:/var/lib/puzzgen-acme:ro" nginx:stable-alpine
port="$(docker port "$name-bootstrap" 80/tcp | cut -d: -f2)"
[[ $(curl -fsS --retry 10 --retry-connrefused --retry-delay 1 -H "Host: $host" "http://127.0.0.1:$port/.well-known/acme-challenge/check") == puzzgen-acme-check ]]
[[ $(curl -sS -o /dev/null -w '%{http_code}' -H "Host: $host" "http://127.0.0.1:$port/api/v1/account/login") == 503 ]]
# Reproduce a transient 404 during readiness: regular --retry does not retry it.
(sleep 2; printf 'delayed-ready' > "$fixture/webroot/.well-known/acme-challenge/delayed") &
writer=$!
[[ $(curl -fsS --max-time 5 --retry 5 --retry-all-errors --retry-delay 1 --retry-max-time 10 -H "Host: $host" "http://127.0.0.1:$port/.well-known/acme-challenge/delayed") == delayed-ready ]]
wait "$writer"
docker run -d --name "$name-tls" -p 127.0.0.1::80 -p 127.0.0.1::443 \
    -v "$root/deploy/nginx.conf:/etc/nginx/conf.d/default.conf:ro" \
    -v "$root/deploy/nginx-proxy.conf:/etc/nginx/snippets/puzzgen-dev-proxy.conf:ro" \
    -v "$fixture/webroot:/var/lib/puzzgen-acme:ro" \
    -v "$fixture/cert:/etc/letsencrypt/live/$host:ro" nginx:stable-alpine
port="$(docker port "$name-tls" 80/tcp | cut -d: -f2)"
tls_port="$(docker port "$name-tls" 443/tcp | cut -d: -f2)"
[[ $(curl -fsS --retry 10 --retry-connrefused --retry-delay 1 -H "Host: $host" "http://127.0.0.1:$port/.well-known/acme-challenge/check") == puzzgen-acme-check ]]
[[ $(curl --noproxy '*' -fsS --cacert "$fixture/cert/fullchain.pem" --resolve "$host:$tls_port:127.0.0.1" "https://$host:$tls_port/.well-known/acme-challenge/check") == puzzgen-acme-check ]]
[[ $(curl -sS -o /dev/null -w '%{http_code}' -H "Host: $host" "http://127.0.0.1:$port/") == 301 ]]
[[ $(curl -sS -o /dev/null -w '%{http_code}' -H "Host: $host" "http://127.0.0.1:$port/.well-known/acme-challenge/missing") == 404 ]]
echo 'nginx routing passed: bootstrap isolates API; ACME works on HTTP and trusted HTTPS; delayed 404 readiness retries; redirect and missing-file behavior correct'
