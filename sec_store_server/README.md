# `sec_store_server`

`sec_store_server` exposes named `sec_store` repositories over HTTPS with mutual TLS.

## Run

```bash
cargo run -p sec_store_server -- \
  --data-dir /tmp/sec-store-server-data \
  --server-cert-pem certs/server.pem \
  --server-key-pem certs/server-key.pem \
  --client-ca-cert-pem certs/ca.pem
```

## Certificate setup

Create a CA:

```bash
openssl genrsa -out ca.key 4096
openssl req -x509 -new -nodes -key ca.key -sha256 -days 3650 \
  -out ca.pem -subj "/CN=PasswordsKeeper Local CA"
```

Create a server certificate:

```bash
openssl genrsa -out server.key 4096
openssl req -new -key server.key -out server.csr -subj "/CN=localhost"
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out server.pem -days 825 -sha256 \
  -extfile <(printf "subjectAltName=DNS:localhost,IP:127.0.0.1")
```

Create a client certificate:

```bash
openssl genrsa -out client.key 4096
openssl req -new -key client.key -out client.csr -subj "/CN=passwordskeeper-client"
openssl x509 -req -in client.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out client.pem -days 825 -sha256
cat client.pem client.key > client-identity.pem
```

Use:

- `ca.pem` as `--client-ca-cert-pem` on the server.
- `server.pem` and `server.key` as the server certificate and key.
- `client-identity.pem` plus `ca.pem` in `sec_store::repository::remote::RemoteClientConfig`.

Clients without a certificate signed by `ca.pem` will be rejected during the TLS handshake.
