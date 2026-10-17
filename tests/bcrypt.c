/* CNG hashes, HMAC and AES through bcrypt.dll. Each result is written to stdout as
   status (u32), length (u32), bytes; tests/test_bcrypt.py checks them. Calls run
   before emit() because C leaves argument evaluation order unspecified. */
#include <windows.h>
#include <bcrypt.h>

void *memset(void *d, int c, size_t n) { unsigned char *p = d; while (n--) *p++ = (unsigned char)c; return d; }
void *memcpy(void *d, const void *s, size_t n) { unsigned char *p = d; const unsigned char *q = s; while (n--) *p++ = *q++; return d; }

static HANDLE out;
static void emit(NTSTATUS status, const void *data, ULONG n) {
    DWORD w;
    ULONG head[2] = {(ULONG)status, n};
    WriteFile(out, head, 8, &w, NULL);
    if (n) WriteFile(out, data, n, &w, NULL);
}
static ULONG length(const char *s) { ULONG n = 0; while (s[n]) n++; return n; }

static const char fox[] = "The quick brown fox jumps over the lazy dog";
static UCHAR key16[16] = {0x2b,0x7e,0x15,0x16,0x28,0xae,0xd2,0xa6,0xab,0xf7,0x15,0x88,0x09,0xcf,0x4f,0x3c};

static void hashes(void) {
    BCRYPT_ALG_HANDLE alg, hmac;
    BCRYPT_HASH_HANDLE h, copy;
    UCHAR digest[32], long_key[100];
    ULONG value = 0, got = 0;
    NTSTATUS status;
    /* SHA-256 of "abc", with a duplicate taken half way. */
    emit(BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA256_ALGORITHM, NULL, 0), NULL, 0);
    status = BCryptGetProperty(alg, BCRYPT_HASH_LENGTH, (PUCHAR)&value, 4, &got, 0);
    emit(status, &value, got);
    BCryptCreateHash(alg, &h, NULL, 0, NULL, 0, 0);
    BCryptHashData(h, (PUCHAR)"a", 1, 0);
    emit(BCryptDuplicateHash(h, &copy, NULL, 0, 0), NULL, 0);
    BCryptHashData(h, (PUCHAR)"bc", 2, 0);
    emit(BCryptFinishHash(h, digest, 32, 0), digest, 32);
    emit(BCryptFinishHash(copy, digest, 32, 0), digest, 32); /* SHA-256("a") */
    emit(BCryptFinishHash(copy, digest, 31, 0), NULL, 0);    /* wrong size */
    BCryptDestroyHash(h);
    BCryptDestroyHash(copy);
    BCryptCloseAlgorithmProvider(alg, 0);
    /* HMAC-SHA256, short key in two chunks, then the same handle reused. */
    emit(BCryptOpenAlgorithmProvider(&hmac, BCRYPT_SHA256_ALGORITHM, NULL, BCRYPT_ALG_HANDLE_HMAC_FLAG), NULL, 0);
    BCryptCreateHash(hmac, &h, NULL, 0, (PUCHAR)"key", 3, 0);
    BCryptHashData(h, (PUCHAR)fox, 10, 0);
    BCryptHashData(h, (PUCHAR)fox + 10, length(fox) - 10, 0);
    emit(BCryptFinishHash(h, digest, 32, 0), digest, 32);
    BCryptHashData(h, (PUCHAR)fox, length(fox), 0);
    emit(BCryptFinishHash(h, digest, 32, 0), digest, 32);
    BCryptDestroyHash(h);
    /* A key longer than the block is hashed first. */
    for (int i = 0; i < 100; i++) long_key[i] = (UCHAR)i;
    BCryptCreateHash(hmac, &h, NULL, 0, long_key, 100, 0);
    BCryptHashData(h, (PUCHAR)fox, length(fox), 0);
    emit(BCryptFinishHash(h, digest, 32, 0), digest, 32);
    BCryptDestroyHash(h);
    BCryptCloseAlgorithmProvider(hmac, 0);
}

static void aes(void) {
    BCRYPT_ALG_HANDLE alg;
    BCRYPT_KEY_HANDLE key;
    UCHAR iv[16], plain[48], cipher[48], buffer[64];
    ULONG n = 0;
    NTSTATUS status;
    for (int i = 0; i < 48; i++) plain[i] = (UCHAR)(i * 7 + 1);
    emit(BCryptOpenAlgorithmProvider(&alg, BCRYPT_AES_ALGORITHM, NULL, 0), NULL, 0);
    emit(BCryptSetProperty(alg, BCRYPT_CHAINING_MODE, (PUCHAR)BCRYPT_CHAIN_MODE_CBC, sizeof(BCRYPT_CHAIN_MODE_CBC), 0), NULL, 0);
    emit(BCryptGenerateSymmetricKey(alg, &key, NULL, 0, key16, 16, 0), NULL, 0);
    /* Padded CBC: size query, encryption of 20 bytes, IV after the call. */
    for (int i = 0; i < 16; i++) iv[i] = (UCHAR)i;
    status = BCryptEncrypt(key, plain, 20, NULL, iv, 16, NULL, 0, &n, BCRYPT_BLOCK_PADDING);
    emit(status, &n, 4);
    status = BCryptEncrypt(key, plain, 20, NULL, iv, 16, cipher, 32, &n, BCRYPT_BLOCK_PADDING);
    emit(status, cipher, n);
    emit(0, iv, 16);
    /* In-place padded decryption with the original IV. */
    for (int i = 0; i < 16; i++) iv[i] = (UCHAR)i;
    memcpy(buffer, cipher, 32);
    status = BCryptDecrypt(key, buffer, 32, NULL, iv, 16, buffer, 32, &n, BCRYPT_BLOCK_PADDING);
    emit(status, buffer, n);
    /* Unpadded CBC on whole blocks, split over two chained calls. */
    for (int i = 0; i < 16; i++) iv[i] = (UCHAR)i;
    status = BCryptEncrypt(key, plain, 16, NULL, iv, 16, cipher, 48, &n, 0);
    emit(status, cipher, n);
    status = BCryptEncrypt(key, plain + 16, 32, NULL, iv, 16, cipher + 16, 32, &n, 0);
    emit(status, cipher + 16, n);
    /* Errors: partial block without padding, small buffer, bad padding. */
    emit(BCryptEncrypt(key, plain, 20, NULL, iv, 16, buffer, 64, &n, 0), NULL, 0);
    emit(BCryptEncrypt(key, plain, 20, NULL, iv, 16, buffer, 16, &n, BCRYPT_BLOCK_PADDING), NULL, 0);
    for (int i = 0; i < 16; i++) iv[i] = (UCHAR)i;
    emit(BCryptDecrypt(key, plain, 32, NULL, iv, 16, buffer, 64, &n, BCRYPT_BLOCK_PADDING) == 0, NULL, 0);
    BCryptDestroyKey(key);
    /* ECB set on the key only. */
    BCryptGenerateSymmetricKey(alg, &key, NULL, 0, key16, 16, 0);
    emit(BCryptSetProperty(key, BCRYPT_CHAINING_MODE, (PUCHAR)BCRYPT_CHAIN_MODE_ECB, sizeof(BCRYPT_CHAIN_MODE_ECB), 0), NULL, 0);
    status = BCryptEncrypt(key, plain, 32, NULL, NULL, 0, cipher, 32, &n, 0);
    emit(status, cipher, n);
    BCryptDestroyKey(key);
    BCryptCloseAlgorithmProvider(alg, 0);
}

static void gcm(void) {
    BCRYPT_ALG_HANDLE alg;
    BCRYPT_KEY_HANDLE key;
    BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO info;
    UCHAR nonce[12], tag[16], plain[24], cipher[24], back[24];
    ULONG n = 0;
    NTSTATUS status;
    for (int i = 0; i < 12; i++) nonce[i] = (UCHAR)(0xa0 + i);
    for (int i = 0; i < 24; i++) plain[i] = (UCHAR)i;
    BCryptOpenAlgorithmProvider(&alg, BCRYPT_AES_ALGORITHM, NULL, 0);
    emit(BCryptSetProperty(alg, BCRYPT_CHAINING_MODE, (PUCHAR)BCRYPT_CHAIN_MODE_GCM, sizeof(BCRYPT_CHAIN_MODE_GCM), 0), NULL, 0);
    BCryptGenerateSymmetricKey(alg, &key, NULL, 0, key16, 16, 0);
    BCRYPT_INIT_AUTH_MODE_INFO(info);
    info.pbNonce = nonce; info.cbNonce = 12;
    info.pbAuthData = (PUCHAR)"header"; info.cbAuthData = 6;
    info.pbTag = tag; info.cbTag = 16;
    status = BCryptEncrypt(key, plain, 24, &info, NULL, 0, cipher, 24, &n, 0);
    emit(status, cipher, n);
    emit(0, tag, 16);
    status = BCryptDecrypt(key, cipher, 24, &info, NULL, 0, back, 24, &n, 0);
    emit(status, back, n);
    tag[0] ^= 1;
    emit(BCryptDecrypt(key, cipher, 24, &info, NULL, 0, back, 24, &n, 0), NULL, 0);
    BCryptDestroyKey(key);
    BCryptCloseAlgorithmProvider(alg, 0);
}

void entry(void) {
    BCRYPT_ALG_HANDLE alg;
    out = GetStdHandle(STD_OUTPUT_HANDLE);
    hashes();
    aes();
    gcm();
    /* Unknown algorithms and properties are refused. */
    emit(BCryptOpenAlgorithmProvider(&alg, L"FOO", NULL, 0), NULL, 0);
    BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA256_ALGORITHM, NULL, 0);
    emit(BCryptSetProperty(alg, L"Bogus", (PUCHAR)"x", 1, 0), NULL, 0);
    BCryptCloseAlgorithmProvider(alg, 0);
    ExitProcess(0);
}
