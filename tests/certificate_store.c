#include <windows.h>
#include <wincrypt.h>
#include <stddef.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
_Static_assert(sizeof(CERT_CONTEXT)==40,"CERT_CONTEXT ABI");
_Static_assert(sizeof(CERT_INFO)==208,"CERT_INFO ABI");
_Static_assert(offsetof(CERT_INFO,SubjectPublicKeyInfo)==96,"public key ABI");
static int equal(const char *a,const char *b){while(*a&&*a==*b){a++;b++;}return *a==*b;}
void entry(void){
    CHECK(!CertOpenStore((LPCSTR)123,0,0,0,0)&&GetLastError()==ERROR_NOT_SUPPORTED);
    HCERTSTORE root=CertOpenStore(CERT_STORE_PROV_SYSTEM_W,0,0,CERT_SYSTEM_STORE_CURRENT_USER|CERT_STORE_READONLY_FLAG,L"ROOT");CHECK(root);
    PCCERT_CONTEXT first=CertEnumCertificatesInStore(root,0);CHECK(first&&first->pCertInfo&&first->pbCertEncoded&&first->cbCertEncoded>100);
    DWORD hash_size=0;BYTE hash[32];
    CHECK(CertGetCertificateContextProperty(first,CERT_SHA1_HASH_PROP_ID,NULL,&hash_size)&&hash_size==20);
    hash_size=1;CHECK(!CertGetCertificateContextProperty(first,CERT_SHA1_HASH_PROP_ID,hash,&hash_size)&&GetLastError()==ERROR_MORE_DATA&&hash_size==20);
    CHECK(CertGetCertificateContextProperty(first,CERT_SHA1_HASH_PROP_ID,hash,&hash_size)&&hash_size==20);
#ifndef HOST_BUNDLE
    HANDLE hashes=CreateFileW(L"hashes.bin",GENERIC_WRITE,0,NULL,CREATE_ALWAYS,0,NULL);CHECK(hashes!=INVALID_HANDLE_VALUE);
    DWORD done;CHECK(WriteFile(hashes,hash,hash_size,&done,NULL)&&done==hash_size);
    hash_size=sizeof(hash);CHECK(CertGetCertificateContextProperty(first,CERT_MD5_HASH_PROP_ID,hash,&hash_size)&&hash_size==16);
    CHECK(WriteFile(hashes,hash,hash_size,&done,NULL)&&done==hash_size);
    hash_size=sizeof(hash);CHECK(CertGetCertificateContextProperty(first,CERT_SHA256_HASH_PROP_ID,hash,&hash_size)&&hash_size==32);
    CHECK(WriteFile(hashes,hash,hash_size,&done,NULL)&&done==hash_size);CHECK(CloseHandle(hashes));
#endif
    CHECK(!CertGetCertificateContextProperty(first,CERT_FRIENDLY_NAME_PROP_ID,NULL,&hash_size)&&GetLastError()==(DWORD)CRYPT_E_NOT_FOUND);
    CHECK(!CertGetCertificateContextProperty(NULL,CERT_SHA1_HASH_PROP_ID,NULL,&hash_size)&&GetLastError()==ERROR_INVALID_PARAMETER);
    CHECK(first->hCertStore==root&&first->dwCertEncodingType==X509_ASN_ENCODING);
    CHECK(first->pCertInfo->Issuer.cbData&&first->pCertInfo->Issuer.pbData[0]==0x30);
    CHECK(first->pCertInfo->Subject.cbData&&first->pCertInfo->Subject.pbData[0]==0x30);
    CHECK(first->pCertInfo->SubjectPublicKeyInfo.PublicKey.cbData>0);
#ifndef HOST_BUNDLE
    CHECK(first->pCertInfo->dwVersion==CERT_V3);
    CHECK(first->pCertInfo->SerialNumber.cbData==2&&first->pCertInfo->SerialNumber.pbData[0]==0x34&&first->pCertInfo->SerialNumber.pbData[1]==0x12);
    CHECK(equal(first->pCertInfo->SignatureAlgorithm.pszObjId,"1.2.840.10045.4.3.2"));
    CHECK(equal(first->pCertInfo->SubjectPublicKeyInfo.Algorithm.pszObjId,"1.2.840.10045.2.1"));
#endif
    PCCERT_CONTEXT kept=CertDuplicateCertificateContext(first);CHECK(kept==first);
    unsigned count=1;PCCERT_CONTEXT current=first;
    while((current=CertEnumCertificatesInStore(root,current)))count++;
    CHECK(GetLastError()==(DWORD)CRYPT_E_NOT_FOUND&&count>0);
    CHECK(!CertCloseStore(root,CERT_CLOSE_STORE_CHECK_FLAG)&&GetLastError()==(DWORD)CRYPT_E_PENDING_CLOSE);
    CHECK(kept->pbCertEncoded[0]==0x30&&kept->pCertInfo->cExtension>0);
    HCERTSTORE memory=CertOpenStore(CERT_STORE_PROV_MEMORY,0,0,0,0);CHECK(memory);
    PCCERT_CONTEXT copy;
    CHECK(CertAddEncodedCertificateToStore(memory,X509_ASN_ENCODING,kept->pbCertEncoded,kept->cbCertEncoded,CERT_STORE_ADD_NEW,&copy));
    CHECK(copy&&copy->hCertStore==memory);
    CHECK(!CertAddEncodedCertificateToStore(memory,X509_ASN_ENCODING,kept->pbCertEncoded,kept->cbCertEncoded,CERT_STORE_ADD_NEW,0)&&GetLastError()==(DWORD)CRYPT_E_EXISTS);
    CHECK(CertFreeCertificateContext(kept));
    CHECK(CertCloseStore(memory,0));CHECK(copy->pbCertEncoded[0]==0x30&&copy->pCertInfo->Issuer.cbData>0);
    PCCERT_CONTEXT standalone=CertCreateCertificateContext(X509_ASN_ENCODING,copy->pbCertEncoded,copy->cbCertEncoded);CHECK(standalone&&standalone->hCertStore==0);
    CHECK(CertFreeCertificateContext(copy));CHECK(CertFreeCertificateContext(standalone));CHECK(CertFreeCertificateContext(0));
    const BYTE invalid[]={0x30,0x80,0,0};CHECK(!CertCreateCertificateContext(X509_ASN_ENCODING,invalid,sizeof(invalid)));
    ExitProcess(0);
}
