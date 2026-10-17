#define COBJMACROS
#include <windows.h>
#include <dwrite_2.h>
#define CHECK(x) do { if(!(x)) ExitProcess(__LINE__); } while(0)
static const GUID factory_iid={0xb859ee5a,0xd838,0x4b5b,{0xa2,0xe8,0x1a,0xdc,0x7d,0x93,0xdb,0x48}};
static const GUID factory2_iid={0x0439fc60,0xca44,0x4994,{0x8d,0xee,0x3a,0x9a,0xf7,0xb7,0x32,0xec}};
static const GUID local_iid={0xb2d9f3ec,0xc9fe,0x4a11,{0xa2,0xec,0xd8,0x62,0x08,0xf7,0xc0,0xa2}};
void entry(void) {
 IDWriteFactory *f=0; IDWriteFontCollection *c=0; IDWriteFontFamily *family=0;
 IDWriteLocalizedStrings *names=0; IDWriteFont *font=0; IDWriteFontFace *face=0;
 IDWriteFontFile *file=0; IDWriteFontFileLoader *loader=0; IDWriteLocalFontFileLoader *local=0;
 CHECK(DWriteCreateFactory(2,&factory_iid,(IUnknown**)&f)==E_INVALIDARG && !f);
 CHECK(DWriteCreateFactory(0,&factory_iid,(IUnknown**)&f)==S_OK && f);
 IDWriteFactory *isolated=0;
 CHECK(DWriteCreateFactory(1,&factory_iid,(IUnknown**)&isolated)==S_OK && isolated!=f);
 IDWriteFactory_Release(isolated);
 CHECK(IDWriteFactory_GetSystemFontCollection(f,&c,FALSE)==S_OK);
 UINT32 count=IDWriteFontCollection_GetFontFamilyCount(c); CHECK(count>0);
 CHECK(IDWriteFontCollection_GetFontFamily(c,0,&family)==S_OK);
 CHECK(IDWriteFontCollection_GetFontFamily(c,count,&family)==E_INVALIDARG && !family);
 CHECK(IDWriteFontCollection_GetFontFamily(c,0,&family)==S_OK);
 CHECK(IDWriteFontFamily_GetFamilyNames(family,&names)==S_OK);
 UINT32 n=0; WCHAR name[1024]; CHECK(IDWriteLocalizedStrings_GetStringLength(names,0,&n)==S_OK && n>0 && n<1024);
 CHECK(IDWriteLocalizedStrings_GetString(names,0,name,n)==HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER));
 CHECK(IDWriteLocalizedStrings_GetString(names,0,name,1024)==S_OK && name[n]==0);
 UINT32 index=99; BOOL exists=FALSE;
 CHECK(IDWriteFontCollection_FindFamilyName(c,name,&index,&exists)==S_OK && exists && index==0);
 CHECK(IDWriteFontFamily_GetFirstMatchingFont(family,400,5,0,&font)==S_OK);
 CHECK(IDWriteFont_CreateFontFace(font,&face)==S_OK);
 n=0; CHECK(IDWriteFontFace_GetFiles(face,&n,NULL)==S_OK && n==1);
 CHECK(IDWriteFontFace_GetFiles(face,&n,&file)==S_OK);
 const void *key=0; UINT32 bytes=0;
 CHECK(IDWriteFontFile_GetReferenceKey(file,&key,&bytes)==S_OK && bytes>2);
 CHECK(IDWriteFontFile_GetLoader(file,&loader)==S_OK);
 CHECK(IDWriteFontFileLoader_QueryInterface(loader,&local_iid,(void**)&local)==S_OK);
 CHECK(IDWriteLocalFontFileLoader_GetFilePathLengthFromKey(local,key,bytes,&n)==S_OK && n<4096);
 WCHAR path[4096]; CHECK(IDWriteLocalFontFileLoader_GetFilePathFromKey(local,key,bytes,path,4096)==S_OK);
 CHECK(GetFileAttributesW(path)!=INVALID_FILE_ATTRIBUTES);
 IDWriteNumberSubstitution *sub=0;
 CHECK(IDWriteFactory_CreateNumberSubstitution(f,DWRITE_NUMBER_SUBSTITUTION_METHOD_NONE,L"en-US",TRUE,&sub)==S_OK);
 CHECK(IDWriteNumberSubstitution_Release(sub)==0);
 CHECK(IDWriteFactory_CreateNumberSubstitution(f,5,L"en-US",TRUE,&sub)==E_INVALIDARG && !sub);
 IDWriteFactory2 *v2=0; IDWriteFontFallback *fallback=0;
 CHECK(IDWriteFactory_QueryInterface(f,&factory2_iid,(void**)&v2)==S_OK);
 CHECK(IDWriteFactory2_GetSystemFontFallback(v2,&fallback)==S_OK && fallback);
 CHECK(IDWriteFontFallback_Release(fallback)>=1);
 IDWriteLocalFontFileLoader_Release(local); IDWriteFontFileLoader_Release(loader);
 IDWriteFontFile_Release(file); IDWriteFontFace_Release(face); IDWriteFont_Release(font);
 IDWriteLocalizedStrings_Release(names); IDWriteFontFamily_Release(family);
 IDWriteFontCollection_Release(c); IDWriteFactory2_Release(v2); IDWriteFactory_Release(f);
 ExitProcess(0);
}
