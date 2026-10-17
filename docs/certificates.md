# CRYPT32 certificate support

The implementation is split between store ownership (`mod.rs`), Windows metadata
layouts and bounded DER parsing (`der.rs`), CA-bundle reading (`pem.rs`), chain and
policy objects (`chain.rs`), and host-only OpenSSL handles (`openssl.rs`).

## Supported behavior

- `CertOpenStore`: in-memory stores and read-only system ROOT snapshots for the
  current-user/local-machine locations. ROOT comes from the Linux CA bundle;
  `SSL_CERT_FILE` can override its file. Unknown stores/providers fail explicitly.
- Enumeration consumes the previous context reference. Duplicate/free operations
  track references; contexts and their backing store survive normal store close.
  The Windows `CERT_CONTEXT` and complete `CERT_INFO` layouts contain owned DER,
  issuer/subject, validity, serial, key/algorithm identifiers and extensions.
- Memory-store additions support NEW, USE_EXISTING, REPLACE_EXISTING and ALWAYS.
  Host roots are never modified. Other dispositions/flags fail explicitly.
- `CertGetCertificateChain` uses host OpenSSL for real signature, trust, time and
  supported EKU checks. Additional-store certificates are untrusted intermediates,
  never new trust anchors. Failed verification is represented in Windows trust
  status bits; successfully building a chain does not mean that it is trusted.
- BASE and SSL policies report trust errors. SSL also checks server/client usage
  and the requested DNS name or IP address. Chain allocations contain only Windows
  layouts and owned certificate contexts; no host object pointers cross the ABI.

Online revocation and AIA fetching are not implemented. Requested revocation
checks report unknown/offline status, which policy rejects. Custom engines,
non-default extra policies and unsupported flags return errors. There is no
accept-all fallback and no automatic suppression of validation errors.

Requires OpenSSL `libcrypto` (development linker files for building). The existing
UnityTLS backend continues to perform TLS; these functions supply its Windows
certificate-validation calls.

## Checks

```sh
cargo build --release --offline
python3 tools/stage_dist.py
python3 tests/test_certificate_store.py
python3 tests/test_certificate_chain.py
```

Tests use a locally generated CA/leaf and isolated trust settings, with no network.
They cover x64 layouts, host-root enumeration, reference lifetime, memory stores,
invalid DER, trusted/untrusted chains, DNS/IP mismatch, expiry, wrong EKU, damaged
signatures and revocation requests. Keys and fixtures live under ignored `cache/`.

The latest private-Xvfb Unity check stayed running for 25 seconds, with no missing
API stop or Curl error 35 in its captured log (`logs/unity-cert-chain.log`). This
is a startup smoke check, not a complete online-play or platform security audit.

Contracts follow [Microsoft's certificate-chain API](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-certgetcertificatechain)
and [OpenSSL verification](https://docs.openssl.org/3.0/man3/X509_verify_cert/).

## Context properties

`CertGetCertificateContextProperty` computes SHA-1 (3), MD5 (4) and SHA-256
(107) over the actual DER certificate using host OpenSSL. It supports size-only
queries and ERROR_MORE_DATA, and validates context lifetime. Unattached extended
properties return CRYPT_E_NOT_FOUND. The store test compares these hashes with
Python's hashlib. This does not change certificate trust or signature validation.
