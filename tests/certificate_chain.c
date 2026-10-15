#include <windows.h>
#include <wincrypt.h>
#include <stddef.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
_Static_assert(sizeof(CERT_CHAIN_CONTEXT)==72,"chain ABI");
_Static_assert(sizeof(CERT_SIMPLE_CHAIN)==40,"simple ABI");
_Static_assert(sizeof(CERT_CHAIN_ELEMENT)==56,"element ABI");
static BYTE encoded[16384];static DWORD encoded_size;
static PCCERT_CONTEXT load(void){HANDLE file=CreateFileW(L"leaf.der",GENERIC_READ,FILE_SHARE_READ,0,OPEN_EXISTING,0,0);CHECK(file!=INVALID_HANDLE_VALUE);CHECK(ReadFile(file,encoded,sizeof(encoded),&encoded_size,0));CHECK(CloseHandle(file));PCCERT_CONTEXT c=CertCreateCertificateContext(X509_ASN_ENCODING,encoded,encoded_size);CHECK(c);return c;}
void entry(void){
    PCCERT_CONTEXT leaf=load();CERT_CHAIN_PARA para={0};para.cbSize=sizeof(para);PCCERT_CHAIN_CONTEXT chain=0;
    CHECK(CertGetCertificateChain(0,leaf,0,0,&para,0,0,&chain));CHECK(chain&&chain->cChain==1&&chain->rgpChain[0]->cElement>0);
    CERT_CHAIN_POLICY_PARA policy={0};policy.cbSize=sizeof(policy);CERT_CHAIN_POLICY_STATUS status={0};status.cbSize=sizeof(status);
    CHECK(CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_BASE,chain,&policy,&status));
#ifdef EXPECT_UNTRUSTED
    CHECK(chain->TrustStatus.dwErrorStatus!=0&&status.dwError!=0);
#else
    CHECK(chain->TrustStatus.dwErrorStatus==0&&status.dwError==0&&chain->rgpChain[0]->cElement==2);
    SSL_EXTRA_CERT_CHAIN_POLICY_PARA ssl={0};ssl.cbSize=sizeof(ssl);ssl.dwAuthType=AUTHTYPE_SERVER;ssl.pwszServerName=L"fixture.test";policy.pvExtraPolicyPara=&ssl;
    CHECK(CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_SSL,chain,&policy,&status)&&status.dwError==0);
    ssl.pwszServerName=L"wrong.test";CHECK(CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_SSL,chain,&policy,&status)&&status.dwError==(DWORD)CERT_E_CN_NO_MATCH);
    ssl.pwszServerName=L"127.0.0.1";CHECK(CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_SSL,chain,&policy,&status)&&status.dwError==0);
    ssl.pwszServerName=L"127.0.0.2";CHECK(CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_SSL,chain,&policy,&status)&&status.dwError==(DWORD)CERT_E_CN_NO_MATCH);
    ssl.dwAuthType=AUTHTYPE_CLIENT;ssl.pwszServerName=0;CHECK(CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_SSL,chain,&policy,&status)&&status.dwError==(DWORD)CERT_E_WRONG_USAGE);
    PCCERT_CHAIN_CONTEXT other=0;ULARGE_INTEGER future;future.LowPart=leaf->pCertInfo->NotAfter.dwLowDateTime;future.HighPart=leaf->pCertInfo->NotAfter.dwHighDateTime;future.QuadPart+=864000000000ULL;
    CHECK(CertGetCertificateChain(0,leaf,(FILETIME*)&future,0,&para,0,0,&other));CHECK(other->TrustStatus.dwErrorStatus&CERT_TRUST_IS_NOT_TIME_VALID);CertFreeCertificateChain(other);
    encoded[encoded_size-1]^=1;PCCERT_CONTEXT tampered=CertCreateCertificateContext(X509_ASN_ENCODING,encoded,encoded_size);CHECK(tampered);
    CHECK(CertGetCertificateChain(0,tampered,0,0,&para,0,0,&other));CHECK(other->TrustStatus.dwErrorStatus&CERT_TRUST_IS_NOT_SIGNATURE_VALID);CertFreeCertificateChain(other);CHECK(CertFreeCertificateContext(tampered));
    CHECK(CertGetCertificateChain(0,leaf,0,0,&para,CERT_CHAIN_REVOCATION_CHECK_CHAIN,0,&other));CHECK(other->TrustStatus.dwErrorStatus&CERT_TRUST_REVOCATION_STATUS_UNKNOWN);
    CHECK(CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_BASE,other,0,&status)&&status.dwError!=0);CertFreeCertificateChain(other);
    CHECK(!CertGetCertificateChain(0,leaf,0,0,&para,0x8000,0,&other)&&GetLastError()==ERROR_NOT_SUPPORTED);
#endif
    CHECK(CertDuplicateCertificateChain(chain)==chain);CertFreeCertificateChain(chain);
    CHECK(chain->rgpChain[0]->rgpElement[0]->pCertContext->pCertInfo!=0);CertFreeCertificateChain(chain);CHECK(CertFreeCertificateContext(leaf));ExitProcess(0);
}
