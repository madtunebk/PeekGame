/* The CRYPT32 sequence PlayFab Party (PartyWin.dll) runs for its DTLS identity:
   X509_NAME encoding, a self-signed certificate, its SHA256 hash and time validity,
   plus the My/Root collection store it searches by SHA1 hash. */
#include <windows.h>
#include <wincrypt.h>
#define CHECK(x) do{if(!(x))ExitProcess(__LINE__);}while(0)
void entry(void){
    static WCHAR cn[]=L"PeekGame Party";
    CERT_RDN_ATTR attr={(LPSTR)"2.5.4.3",CERT_RDN_UTF8_STRING,{sizeof(cn)-2,(BYTE*)cn}};
    CERT_RDN rdn={1,&attr};CERT_NAME_INFO info={1,&rdn};
    DWORD size=0;
    CHECK(CryptEncodeObjectEx(X509_ASN_ENCODING|PKCS_7_ASN_ENCODING,X509_NAME,&info,0,NULL,NULL,&size)&&size==27);
    BYTE name[64];DWORD small=4;
    CHECK(!CryptEncodeObjectEx(X509_ASN_ENCODING,X509_NAME,&info,0,NULL,name,&small)&&GetLastError()==ERROR_MORE_DATA&&small==27);
    size=sizeof(name);CHECK(CryptEncodeObjectEx(X509_ASN_ENCODING,X509_NAME,&info,0,NULL,name,&size)&&size==27);
    CHECK(name[0]==0x30&&name[11]==0x0c&&name[12]==14&&name[13]=='P');
    BYTE *allocated=NULL;DWORD allocated_size=0;
    CHECK(CryptEncodeObjectEx(X509_ASN_ENCODING,X509_NAME,&info,CRYPT_ENCODE_ALLOC_FLAG,NULL,&allocated,&allocated_size)&&allocated&&allocated_size==27&&allocated[13]=='P');
    LocalFree(allocated);
    DWORD other=0;CHECK(!CryptEncodeObjectEx(X509_ASN_ENCODING,X509_PUBLIC_KEY_INFO,&info,0,NULL,NULL,&other)&&GetLastError()==ERROR_NOT_SUPPORTED);

    CERT_NAME_BLOB subject={size,name};
    SYSTEMTIME start={2026,1,4,1,0,0,0,0},end={2027,1,5,1,0,0,0,0};
    CHECK(!CertCreateSelfSignCertificate(0,&subject,CERT_CREATE_SELFSIGN_NO_SIGN,NULL,NULL,NULL,NULL,NULL)&&GetLastError()==ERROR_NOT_SUPPORTED);
    PCCERT_CONTEXT cert=CertCreateSelfSignCertificate(0,&subject,0,NULL,NULL,&start,&end,NULL);
    CHECK(cert&&cert->pbCertEncoded[0]==0x30&&cert->pCertInfo->dwVersion==CERT_V3);
    CHECK(cert->pCertInfo->Subject.cbData==27&&cert->pCertInfo->Issuer.cbData==27);
    for(int i=0;i<27;i++)CHECK(cert->pCertInfo->Subject.pbData[i]==name[i]);
    BYTE hash[32];DWORD hash_size=sizeof(hash);
    CHECK(CryptHashCertificate2(L"SHA256",0,NULL,cert->pbCertEncoded,cert->cbCertEncoded,hash,&hash_size)&&hash_size==32);
    hash_size=8;CHECK(!CryptHashCertificate2(L"SHA256",0,NULL,cert->pbCertEncoded,cert->cbCertEncoded,hash,&hash_size)&&GetLastError()==ERROR_MORE_DATA&&hash_size==32);
    CHECK(!CryptHashCertificate2(L"NOPE",0,NULL,cert->pbCertEncoded,cert->cbCertEncoded,hash,&hash_size)&&GetLastError()==(DWORD)NTE_BAD_ALGID);
    FILETIME inside,before,after;SYSTEMTIME t={2026,6,1,1,12,0,0,0};
    CHECK(SystemTimeToFileTime(&t,&inside));t.wYear=2025;CHECK(SystemTimeToFileTime(&t,&before));t.wYear=2028;CHECK(SystemTimeToFileTime(&t,&after));
    CHECK(CertVerifyTimeValidity(&inside,cert->pCertInfo)==0);
    CHECK(CertVerifyTimeValidity(&before,cert->pCertInfo)==-1);
    CHECK(CertVerifyTimeValidity(&after,cert->pCertInfo)==1);
    HANDLE out=CreateFileW(L"selfsigned.der",GENERIC_WRITE,0,NULL,CREATE_ALWAYS,0,NULL);CHECK(out!=INVALID_HANDLE_VALUE);
    DWORD done;CHECK(WriteFile(out,cert->pbCertEncoded,cert->cbCertEncoded,&done,NULL)&&CloseHandle(out));
    PCCERT_CONTEXT now=CertCreateSelfSignCertificate(0,&subject,0,NULL,NULL,NULL,NULL,NULL);
    CHECK(now&&CertVerifyTimeValidity(NULL,now->pCertInfo)==0);
    CHECK(CertFreeCertificateContext(now));

    HCERTSTORE collection=CertOpenStore(CERT_STORE_PROV_COLLECTION,0,0,CERT_STORE_DEFER_CLOSE_UNTIL_LAST_FREE_FLAG,NULL);CHECK(collection);
    static const struct{const WCHAR*name;DWORD flags;}stores[]={{L"My",CERT_SYSTEM_STORE_CURRENT_USER},{L"Root",CERT_SYSTEM_STORE_CURRENT_USER},{L"My",CERT_SYSTEM_STORE_LOCAL_MACHINE},{L"Root",CERT_SYSTEM_STORE_LOCAL_MACHINE}};
    for(int i=0;i<4;i++){
        HCERTSTORE sibling=CertOpenStore(CERT_STORE_PROV_SYSTEM_W,0,0,stores[i].flags|CERT_STORE_OPEN_EXISTING_FLAG|CERT_STORE_READONLY_FLAG|CERT_STORE_DEFER_CLOSE_UNTIL_LAST_FREE_FLAG,stores[i].name);
        CHECK(sibling);CHECK(CertAddStoreToCollection(collection,sibling,0,0));CHECK(CertCloseStore(sibling,0));
    }
    PCCERT_CONTEXT first=CertEnumCertificatesInStore(collection,NULL);CHECK(first);
    BYTE sha1[20];DWORD sha1_size=sizeof(sha1);
    CHECK(CertGetCertificateContextProperty(first,CERT_SHA1_HASH_PROP_ID,sha1,&sha1_size)&&sha1_size==20);
    CRYPT_HASH_BLOB wanted={20,sha1};
    PCCERT_CONTEXT found=CertFindCertificateInStore(collection,X509_ASN_ENCODING|PKCS_7_ASN_ENCODING,0,CERT_FIND_SHA1_HASH,&wanted,NULL);
    CHECK(found&&found->cbCertEncoded==first->cbCertEncoded);
    CHECK(CertFreeCertificateContext(found));CHECK(CertFreeCertificateContext(first));
    sha1[0]^=0xff;
    CHECK(!CertFindCertificateInStore(collection,X509_ASN_ENCODING,0,CERT_FIND_SHA1_HASH,&wanted,NULL)&&GetLastError()==(DWORD)CRYPT_E_NOT_FOUND);
    CHECK(!CertFindCertificateInStore(collection,X509_ASN_ENCODING,0,CERT_FIND_SUBJECT_STR_W,L"x",NULL)&&GetLastError()==ERROR_NOT_SUPPORTED);
    CHECK(!CertOpenStore(CERT_STORE_PROV_SYSTEM_W,0,0,CERT_SYSTEM_STORE_CURRENT_USER,L"My")&&GetLastError()==ERROR_NOT_SUPPORTED);
    CHECK(CertCloseStore(collection,0));
    CHECK(CertFreeCertificateContext(cert));
    ExitProcess(0);
}
