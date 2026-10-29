# Development

Each server is its own Docker Compose project in its own directory, with a `.env` (and an optional git-ignored `.env.local`). Per-server details (logging, `.sqlx` metadata, migrations): [identity-provider](identity-provider/DEV.md), [auth-server](auth-server/DEV.md).

| Server               | Directory            | Host port |
| -------------------- | -------------------- | --------- |
| identity-provider    | `identity-provider/` | 8080      |
| auth-server          | `auth-server/`       | 8081      |
| spec-provider (gRPC) | `spec-provider/`     | 50051     |
| spec-viewer          | `spec-viewer/`       | 8090      |

## HTTPS

Every server terminates TLS itself and only answers over https. It exits at startup without `TLS_CERT_PATH` and `TLS_KEY_PATH` (PEM files, set in each `.env`). Only the databases and redis speak plain protocols, on the internal Docker networks.

Servers reach each other by domain name, so every URL is the same inside and outside the containers:

| Server               | URL                              |
| -------------------- | -------------------------------- |
| identity-provider    | `https://identity-provider.test` |
| auth-server          | `https://auth-server.test`       |
| spec-provider (gRPC) | `https://spec-provider.test`     |
| spec-viewer          | `https://spec-viewer.test`       |

All names resolve to the host, where the nginx proxy in `local-dev/` listens on 443. It routes by the server name in the TLS handshake (SNI) and passes the encrypted connection through, so it never sees the traffic and the servers keep their own certificates. See `local-dev/nginx.conf`. The servers also publish their own ports (8080, 8081, 50051, 8090) for direct access, e.g. `https://localhost:8081`.

In a container, `extra_hosts: host-gateway` maps the `.test` names of the servers it calls to the host, where the proxy listens. Its `.env` sets `SSL_CERT_FILE=/ca/rootCA.pem`, and the compose file mounts the host's mkcert CA (public part only) there. Clients must read the system roots: reqwest needs `rustls-tls-native-roots`, not `rustls-tls`, which bundles its own and ignores `SSL_CERT_FILE`. `MKCERT_CAROOT` defaults to the macOS location in each `.env`. On Linux, export `MKCERT_CAROOT="$(mkcert -CAROOT)"` in your shell, which takes precedence (`local-dev/dev.sh` does it for you).

## One-time setup

1. auth-server signs ID tokens with an RSA key and exits without one:

   ```sh
   mkdir -p auth-server/keys
   openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out auth-server/keys/signing-key.pem
   ```

2. Create the [TLS certificates](#tls-certificates).
3. Map the names to the local machine:

   ```sh
   echo "127.0.0.1 auth-server.test identity-provider.test spec-provider.test spec-viewer.test" | sudo tee -a /etc/hosts
   ```

   On macOS, flush the DNS cache if that isn't picked up: `sudo dscacheutil -flushcache && sudo killall -HUP mDNSResponder`.

## One server

Run in the server's directory. Start order: identity-provider, auth-server, spec-provider, spec-viewer. auth-server needs the identity-provider running, spec-viewer needs the spec-provider. Servers call each other through the proxy, which only exists in `local-dev/`: on its own, a server works only for requests that don't involve another one.

```sh
docker compose -f docker-compose.yml up -d --build                               # local production
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --watch        # dev, live reload
docker compose -f docker-compose.yml down                                        # add -v to delete data
```

spec-viewer also needs a content service. Without one, run the mock (plain http, on the host): `cargo run --example mock_content` in `spec-viewer/`.

## All servers (`local-dev/`)

`local-dev/docker-compose.yml` runs all four servers and the proxy in one Compose project. Use this or the per-server stacks, not both (ports clash, volumes are separate). Run from `local-dev/`:

```sh
./dev.sh                                                       # choose the servers to run in dev (live reload)
docker compose up -d --build                                   # all local production
DEV_AUTH_SERVER=../auth-server/docker-compose.dev.yml docker compose up --watch   # auth-server in dev
docker compose --env-file dev.env up --watch                   # all in dev
docker compose down                                            # same DEV_* variables as `up`
```

A server runs in dev when its variable (`DEV_IDENTITY_PROVIDER`, `DEV_AUTH_SERVER`) points at its `docker-compose.dev.yml`. `dev.env` sets all.

## TLS certificates

[mkcert](https://github.com/FiloSottile/mkcert) creates a local CA that your system trusts and issues certificates signed by it. Each server keeps its certificate and key in its own `certs/` directory, which its compose file mounts at `/certs`.

```sh
brew install mkcert
brew install nss     # only for Firefox
mkcert -install      # creates and trusts the local CA, once per machine

for s in auth-server identity-provider; do
  mkdir -p $s/certs
  mkcert -cert-file $s/certs/$s.pem -key-file $s/certs/$s-key.pem $s.test localhost 127.0.0.1 ::1
done
```

- Every name a client uses must be in the certificate (`localhost` is for the container health checks). Inspect with `openssl x509 -in auth-server/certs/auth-server.pem -noout -text | grep -A1 "Subject Alternative Name"`.
- Use the `.test` TLD. `.dev` is HSTS-preloaded and `.local` conflicts with mDNS.
- Every `certs/` directory is git-ignored. Never commit `*-key.pem`, and never share `rootCA-key.pem` (`mkcert -CAROOT` shows where it lives). Other devices only need `rootCA.pem`.

### Removing them

```sh
rm -rf {auth-server,identity-provider}/certs   # the certificates and their keys
sudo sed -i '' '/^127.0.0.1 auth-server.test /d' /etc/hosts               # the domains (macOS; the line added in the setup above)
mkcert -uninstall                                                        # remove the CA from the trust stores
rm -rf "$(mkcert -CAROOT)"                                               # delete the CA and its key
```

Without the last two the CA stays trusted, so only remove the certificates if you intend to re-issue them. After deleting the CA, issue new certificates.

## Tests

```sh
cargo test      # in a server's directory (unit), or in e2e-tests/ against the running stack
```
