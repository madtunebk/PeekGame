/* Local fontconfig backend adapted from the owner's peek implementation.
 * No includes or runtime dependencies on the previous project. */
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <pthread.h>
#include <sys/stat.h>
#define WIN __attribute__((ms_abi))
typedef int32_t HRes;
#define E_POINTER_COM ((HRes)0x80004003)
#define E_NOINTERFACE_COM ((HRes)0x80004002)
#define E_INVALIDARG_COM ((HRes)0x80070057)
#define E_OUTOFMEMORY_COM ((HRes)0x8007000e)
#define S_OK_ ((HRes)0)
extern int WIN peekgame_dwrite_multi(uint32_t,uint32_t,const char *,int,uint16_t *,int);
extern int WIN peekgame_dwrite_wide(uint32_t,uint32_t,const uint16_t *,int,char *,int,const char *,int *);
#define multi_byte_to_wide peekgame_dwrite_multi
#define wide_to_multi_byte peekgame_dwrite_wide
static _Noreturn void fail(const char *message) {
    fprintf(stderr,"STOP: %s\n",message); fflush(stderr); _Exit(1);
}
/* DirectWrite (DWrite.dll): the system font collection, from fontconfig.
 * Scope, following what DWriteCore keeps on non-Windows platforms: IDWriteFactory with
 * GetSystemFontCollection; collection -> families -> fonts -> font faces -> font files with the
 * local file loader (file paths are the host's). TrueType/OpenType/CFF files only, as DirectWrite.
 * Every other method (layout, rendering, glyph metrics, font tables, custom collections, GDI
 * interop) stops with its interface and method name: no fake results.
 * Objects of the system collection live for the process; AddRef/Release only count. */
#include <fontconfig/fontconfig.h>

static const unsigned char dw_iid_unknown[16]={0,0,0,0,0,0,0,0,0xc0,0,0,0,0,0,0,0x46};
static const unsigned char dw_iid_factory[16]={0x5a,0xee,0x59,0xb8,0x38,0xd8,0x5b,0x4b,0xa2,0xe8,0x1a,0xdc,0x7d,0x93,0xdb,0x48};
static const unsigned char dw_iid_collection[16]={0x02,0xee,0x4c,0xa8,0xea,0x3e,0xee,0x4e,0xa8,0x27,0x87,0xc1,0xa0,0x2a,0x0f,0xcc};
static const unsigned char dw_iid_family[16]={0xef,0xd8,0x20,0xda,0x2a,0x81,0x43,0x4c,0x98,0x02,0x62,0xec,0x4a,0xbd,0x7a,0xdd};
static const unsigned char dw_iid_font_list[16]={0x38,0x84,0x0d,0x1a,0x97,0x1d,0xc1,0x4e,0xae,0xf9,0xa2,0xfb,0x86,0xed,0x6a,0xcb};
static const unsigned char dw_iid_font[16]={0x96,0x66,0xd1,0xac,0x14,0x8c,0x5d,0x4f,0x87,0x7e,0xfe,0x3f,0xc1,0xd3,0x27,0x37};
static const unsigned char dw_iid_face[16]={0x4d,0x80,0x49,0x5f,0x24,0x70,0x43,0x4d,0xbf,0xa9,0xd2,0x59,0x84,0xf5,0x38,0x49};
static const unsigned char dw_iid_file[16]={0x6a,0x88,0x9d,0x73,0xf5,0xce,0xdc,0x47,0x87,0x69,0x1a,0x8b,0x41,0xbe,0xbb,0xb0};
static const unsigned char dw_iid_file_loader[16]={0x4e,0xad,0x7c,0x72,0xaf,0xd6,0x9e,0x4c,0x8a,0x08,0xd6,0x95,0xb1,0x1c,0xaa,0x49};
static const unsigned char dw_iid_local_loader[16]={0xec,0xf3,0xd9,0xb2,0xfe,0xc9,0x11,0x4a,0xa2,0xec,0xd8,0x62,0x08,0xf7,0xc0,0xa2};
static const unsigned char dw_iid_factory1[16]={0x99,0x2f,0x57,0x30,0xc6,0xda,0xdb,0x41,0xa1,0x6e,0x04,0x86,0x30,0x7e,0x60,0x6a};
static const unsigned char dw_iid_factory2[16]={0x60,0xfc,0x39,0x04,0x44,0xca,0x94,0x49,0x8d,0xee,0x3a,0x9a,0xf7,0xb7,0x32,0xec};
static const unsigned char dw_iid_fallback[16]={0xf9,0x08,0xa0,0xef,0xa1,0xf7,0xbf,0x48,0xb0,0x5c,0xf2,0x24,0x71,0x3c,0xc0,0xff};
static const unsigned char dw_iid_strings[16]={0x09,0x62,0x25,0x08,0x9a,0x09,0x34,0x4b,0xb8,0x6d,0xc2,0x2b,0x11,0x0e,0x77,0x71};
#define DW_E_NOT_SUFFICIENT_BUFFER ((HRes)0x8007007A)

typedef struct {void **vt;uint32_t refs;} DwObject;
typedef struct {void **vt;uint32_t refs;const char *text;} DwStrings;           /* one string, locale en-us */
struct DwFamily;
typedef struct DwFont {
    void **vt;uint32_t refs;struct DwFamily *family;char *path;int index,type;
    uint32_t weight,stretch,style;DwStrings face_names;DwObject face,file;uint16_t *key;uint32_t key_bytes;
    FcCharSet *charset;int charset_loaded;   /* coverage, fetched from fontconfig on first use */
} DwFont;
typedef struct DwFamily {void **vt;uint32_t refs;char *name;DwFont **fonts;uint32_t count;DwStrings names;} DwFamily;
static void *dw_fallback_vt[];
static void *dw_factory_vt[],*dw_collection_vt[],*dw_family_vt[],*dw_font_vt[],*dw_face_vt[],*dw_file_vt[],*dw_loader_vt[],*dw_strings_vt[];
static DwObject dw_fallback={dw_fallback_vt,1};
static DwObject dw_factory={dw_factory_vt,1},dw_collection={dw_collection_vt,1},dw_loader={dw_loader_vt,1};
static DwFamily **dw_families;
static uint32_t dw_family_count;
static int dw_ready;
static pthread_mutex_t dw_mutex=PTHREAD_MUTEX_INITIALIZER;

#define DW_STOP(iface,method) static void WIN dw_stop_##iface##_##method(void) { \
    fprintf(stderr,"DirectWrite: " #iface "::" #method " is not implemented\n");fflush(NULL);fail("DirectWrite method"); }
static int dw_iid_is(const void *iid,const unsigned char *expected) {return iid && !memcmp(iid,expected,16);}
/* Containing structure of an embedded object (face/file/names inside a DwFont). */
#define DW_OUTER(ptr,type,member) ((type *)((char *)(ptr)-offsetof(type,member)))

/* ---- building the collection from fontconfig ---- */
static uint32_t dw_stretch(int width) {   /* FC_WIDTH (50..200) -> DWRITE_FONT_STRETCH 1..9 */
    static const int widths[9]={50,63,75,87,100,113,125,150,200};
    uint32_t best=5;int diff=1000;
    for(int i=0;i<9;i++){int d=abs(width-widths[i]);if(d<diff){diff=d;best=(uint32_t)i+1;}}
    return best;
}
static int dw_compare_family(const void *a,const void *b) {return strcasecmp((*(DwFamily *const *)a)->name,(*(DwFamily *const *)b)->name);}
static int dw_build(void) {
    if(dw_ready)return dw_ready>0;
    dw_ready=-1;
    if(!FcInit()){fprintf(stderr,"DirectWrite: fontconfig initialisation failed\n");return 0;}
    FcPattern *pattern=FcPatternCreate();
    FcObjectSet *objects=FcObjectSetBuild(FC_FAMILY,FC_STYLE,FC_WEIGHT,FC_SLANT,FC_WIDTH,FC_FILE,FC_INDEX,FC_FONTFORMAT,(char *)NULL);
    FcFontSet *set=pattern && objects?FcFontList(NULL,pattern,objects):NULL;
    if(objects)FcObjectSetDestroy(objects);
    if(pattern)FcPatternDestroy(pattern);
    if(!set){fprintf(stderr,"DirectWrite: fontconfig could not list fonts\n");return 0;}
    for(int i=0;i<set->nfont;i++) {
        FcPattern *p=set->fonts[i];FcChar8 *family,*file,*format,*style=NULL;int index=0,weight=FC_WEIGHT_REGULAR,slant=0,width=100;
        if(FcPatternGetString(p,FC_FAMILY,0,&family)!=FcResultMatch || FcPatternGetString(p,FC_FILE,0,&file)!=FcResultMatch)continue;
        if(FcPatternGetString(p,FC_FONTFORMAT,0,&format)!=FcResultMatch ||
           (strcmp((char *)format,"TrueType") && strcmp((char *)format,"CFF")))continue;   /* DirectWrite: OpenType only */
        FcPatternGetString(p,FC_STYLE,0,&style);FcPatternGetInteger(p,FC_INDEX,0,&index);
        FcPatternGetInteger(p,FC_WEIGHT,0,&weight);FcPatternGetInteger(p,FC_SLANT,0,&slant);FcPatternGetInteger(p,FC_WIDTH,0,&width);
        DwFamily *fam=NULL;
        for(uint32_t f=0;f<dw_family_count && !fam;f++)if(!strcmp(dw_families[f]->name,(char *)family))fam=dw_families[f];
        if(!fam) {
            DwFamily **grown=realloc(dw_families,(dw_family_count+1)*sizeof *grown);
            fam=calloc(1,sizeof *fam);
            if(!grown || !fam)fail("DirectWrite: out of memory");
            dw_families=grown;dw_families[dw_family_count++]=fam;
            fam->vt=dw_family_vt;fam->refs=1;fam->name=strdup((char *)family);
            fam->names=(DwStrings){dw_strings_vt,1,fam->name};
        }
        DwFont *font=calloc(1,sizeof *font);
        DwFont **fonts=realloc(fam->fonts,(fam->count+1)*sizeof *fonts);
        if(!font || !fonts)fail("DirectWrite: out of memory");
        fam->fonts=fonts;fam->fonts[fam->count++]=font;
        font->vt=dw_font_vt;font->refs=1;font->family=fam;font->path=strdup((char *)file);font->index=index;
        const char *dot=strrchr(font->path,'.');
        font->type=dot && (!strcasecmp(dot,".ttc") || !strcasecmp(dot,".otc"))?2:!strcmp((char *)format,"CFF")?0:1;
        int ot=FcWeightToOpenType(weight);font->weight=ot>0?(uint32_t)ot:400;
        font->stretch=dw_stretch(width);font->style=slant>=FC_SLANT_OBLIQUE?1:slant>=FC_SLANT_ITALIC?2:0;
        font->face_names=(DwStrings){dw_strings_vt,1,strdup(style?(char *)style:"Regular")};
        font->face=(DwObject){dw_face_vt,1};font->file=(DwObject){dw_file_vt,1};
        int n=multi_byte_to_wide(65001,0,font->path,-1,NULL,0);
        font->key=calloc((size_t)(n>0?n:1),2);if(!font->key)fail("DirectWrite: out of memory");
        multi_byte_to_wide(65001,0,font->path,-1,font->key,n);font->key_bytes=(uint32_t)n*2;
    }
    FcFontSetDestroy(set);
    qsort(dw_families,dw_family_count,sizeof *dw_families,dw_compare_family);
    fprintf(stderr,"DirectWrite: system font collection from fontconfig, %u families\n",dw_family_count);
    dw_ready=1;return 1;
}

/* ---- shared IUnknown parts ---- */
static uint32_t WIN dw_addref(DwObject *o) {return __atomic_add_fetch(&o->refs,1,__ATOMIC_RELAXED);}
static uint32_t WIN dw_release(DwObject *o) {uint32_t n=__atomic_load_n(&o->refs,__ATOMIC_RELAXED);while(n>1 && !__atomic_compare_exchange_n(&o->refs,&n,n-1,0,__ATOMIC_RELAXED,__ATOMIC_RELAXED)){}return n>1?n-1:1;}  /* process lifetime */
static HRes dw_query(void *self,const void *iid,void **out,const unsigned char *a,const unsigned char *b) {
    if(!out)return E_POINTER_COM;
    *out=NULL;
    if(!dw_iid_is(iid,dw_iid_unknown) && !dw_iid_is(iid,a) && !(b && dw_iid_is(iid,b)))return E_NOINTERFACE_COM;
    dw_addref(self);*out=self;return S_OK_;
}
static HRes dw_out(void *object,void **out) {if(!out)return E_POINTER_COM;dw_addref(object);*out=object;return S_OK_;}

/* ---- IDWriteFactory ---- */
/* One factory object answers IDWriteFactory, IDWriteFactory1 and IDWriteFactory2 (one vtable). */
static HRes WIN dw_factory_qi(void *s,const void *iid,void **out) {
    if(dw_iid_is(iid,dw_iid_factory1) || dw_iid_is(iid,dw_iid_factory2))return dw_query(s,iid,out,iid,NULL);
    return dw_query(s,iid,out,dw_iid_factory,NULL);
}
static HRes WIN dw_get_system_font_collection(void *s,void **out,int check_updates) {(void)s;(void)check_updates;return dw_out(&dw_collection,out);}
DW_STOP(IDWriteFactory,CreateCustomFontCollection) DW_STOP(IDWriteFactory,RegisterFontCollectionLoader)
DW_STOP(IDWriteFactory,UnregisterFontCollectionLoader) DW_STOP(IDWriteFactory,CreateFontFileReference)
DW_STOP(IDWriteFactory,CreateCustomFontFileReference) DW_STOP(IDWriteFactory,CreateFontFace)
DW_STOP(IDWriteFactory,CreateRenderingParams) DW_STOP(IDWriteFactory,CreateMonitorRenderingParams)
DW_STOP(IDWriteFactory,CreateCustomRenderingParams) DW_STOP(IDWriteFactory,RegisterFontFileLoader)
DW_STOP(IDWriteFactory,UnregisterFontFileLoader) DW_STOP(IDWriteFactory,CreateTextFormat)
DW_STOP(IDWriteFactory,CreateTypography) DW_STOP(IDWriteFactory,GetGdiInterop) DW_STOP(IDWriteFactory,CreateTextLayout)
DW_STOP(IDWriteFactory,CreateGdiCompatibleTextLayout) DW_STOP(IDWriteFactory,CreateEllipsisTrimmingSign)
DW_STOP(IDWriteFactory,CreateTextAnalyzer) DW_STOP(IDWriteFactory,CreateGlyphRunAnalysis)
/* IDWriteNumberSubstitution: only IUnknown; it carries the substitution method and locale to
 * text analysis. Each one is its own object, freed on its last Release. */
static const unsigned char dw_iid_number_substitution[16]={0xc9,0x5c,0x88,0x14,0xb0,0xba,0x90,0x4f,0xb6,0xed,0x5c,0x36,0x6a,0x2c,0xd0,0x3d};
typedef struct {void **vt;uint32_t refs,method;int ignore_user_override;uint16_t locale[86];} DwNumberSubstitution;
static void *dw_number_substitution_vt[];
static HRes WIN dw_number_substitution_qi(DwNumberSubstitution *n,const void *iid,void **out) {
    if(!out)return E_POINTER_COM;
    *out=NULL;
    if(!dw_iid_is(iid,dw_iid_unknown) && !dw_iid_is(iid,dw_iid_number_substitution))return E_NOINTERFACE_COM;
    __atomic_add_fetch(&n->refs,1,__ATOMIC_RELAXED);*out=n;return S_OK_;
}
static uint32_t WIN dw_number_substitution_addref(DwNumberSubstitution *n) {return __atomic_add_fetch(&n->refs,1,__ATOMIC_RELAXED);}
static uint32_t WIN dw_number_substitution_release(DwNumberSubstitution *n) {
    uint32_t left=__atomic_sub_fetch(&n->refs,1,__ATOMIC_ACQ_REL);if(!left)free(n);return left;
}
static void *dw_number_substitution_vt[]={dw_number_substitution_qi,dw_number_substitution_addref,dw_number_substitution_release};
/* CreateNumberSubstitution: DWRITE_NUMBER_SUBSTITUTION_METHOD 0..4 (FROM_CULTURE, CONTEXTUAL, NONE,
 * NATIONAL, TRADITIONAL) and a locale name (at most LOCALE_NAME_MAX_LENGTH, 85 characters). */
static HRes WIN dw_create_number_substitution(void *s,uint32_t method,const uint16_t *locale,int ignore_user_override,void **out) {
    (void)s;if(!out)return E_POINTER_COM;
    *out=NULL;
    if(method>4 || !locale)return E_INVALIDARG_COM;
    size_t n=0;while(locale[n] && n<85)n++;
    if(locale[n])return E_INVALIDARG_COM;
    DwNumberSubstitution *sub=calloc(1,sizeof *sub);if(!sub)return E_OUTOFMEMORY_COM;
    sub->vt=dw_number_substitution_vt;sub->refs=1;sub->method=method;sub->ignore_user_override=ignore_user_override!=0;
    memcpy(sub->locale,locale,n*2);
    *out=sub;return S_OK_;
}
DW_STOP(IDWriteFactory1,GetEudcFontCollection) DW_STOP(IDWriteFactory1,CreateCustomRenderingParams)
DW_STOP(IDWriteFactory2,CreateFontFallbackBuilder) DW_STOP(IDWriteFactory2,TranslateColorGlyphRun)
DW_STOP(IDWriteFactory2,CreateCustomRenderingParams) DW_STOP(IDWriteFactory2,CreateGlyphRunAnalysis)
static HRes WIN dw_get_system_font_fallback(void *s,void **out) {(void)s;return dw_out(&dw_fallback,out);}
static void *dw_factory_vt[]={dw_factory_qi,dw_addref,dw_release,dw_get_system_font_collection,
    dw_stop_IDWriteFactory_CreateCustomFontCollection,dw_stop_IDWriteFactory_RegisterFontCollectionLoader,
    dw_stop_IDWriteFactory_UnregisterFontCollectionLoader,dw_stop_IDWriteFactory_CreateFontFileReference,
    dw_stop_IDWriteFactory_CreateCustomFontFileReference,dw_stop_IDWriteFactory_CreateFontFace,
    dw_stop_IDWriteFactory_CreateRenderingParams,dw_stop_IDWriteFactory_CreateMonitorRenderingParams,
    dw_stop_IDWriteFactory_CreateCustomRenderingParams,dw_stop_IDWriteFactory_RegisterFontFileLoader,
    dw_stop_IDWriteFactory_UnregisterFontFileLoader,dw_stop_IDWriteFactory_CreateTextFormat,dw_stop_IDWriteFactory_CreateTypography,
    dw_stop_IDWriteFactory_GetGdiInterop,dw_stop_IDWriteFactory_CreateTextLayout,dw_stop_IDWriteFactory_CreateGdiCompatibleTextLayout,
    dw_stop_IDWriteFactory_CreateEllipsisTrimmingSign,dw_stop_IDWriteFactory_CreateTextAnalyzer,
    dw_create_number_substitution,dw_stop_IDWriteFactory_CreateGlyphRunAnalysis,
    dw_stop_IDWriteFactory1_GetEudcFontCollection,dw_stop_IDWriteFactory1_CreateCustomRenderingParams,
    dw_get_system_font_fallback,dw_stop_IDWriteFactory2_CreateFontFallbackBuilder,dw_stop_IDWriteFactory2_TranslateColorGlyphRun,
    dw_stop_IDWriteFactory2_CreateCustomRenderingParams,dw_stop_IDWriteFactory2_CreateGlyphRunAnalysis};

/* ---- IDWriteFontCollection ---- */
static HRes WIN dw_collection_qi(void *s,const void *iid,void **out) {return dw_query(s,iid,out,dw_iid_collection,NULL);}
static uint32_t WIN dw_collection_count(void *s) {(void)s;return dw_family_count;}
static HRes WIN dw_collection_family(void *s,uint32_t index,void **out) {
    (void)s;if(out)*out=NULL;
    if(index>=dw_family_count)return E_INVALIDARG_COM;
    return dw_out(dw_families[index],out);
}
static HRes WIN dw_collection_find(void *s,const uint16_t *name,uint32_t *index,int32_t *exists) {
    (void)s;if(!name || !index || !exists)return E_POINTER_COM;
    char utf8[512];
    if(wide_to_multi_byte(65001,0,name,-1,utf8,sizeof utf8,NULL,NULL)<=0)return E_INVALIDARG_COM;
    *index=UINT32_MAX;*exists=0;
    for(uint32_t i=0;i<dw_family_count;i++)if(!strcasecmp(dw_families[i]->name,utf8)){*index=i;*exists=1;break;}
    return S_OK_;
}
static HRes WIN dw_collection_from_face(void *s,void *face,void **out) {
    (void)s;if(!face)return E_INVALIDARG_COM;
    return dw_out(DW_OUTER(face,DwFont,face),out);
}
static void *dw_collection_vt[]={dw_collection_qi,dw_addref,dw_release,dw_collection_count,dw_collection_family,
    dw_collection_find,dw_collection_from_face};

/* ---- IDWriteFontFamily (an IDWriteFontList) ---- */
static HRes WIN dw_family_qi(void *s,const void *iid,void **out) {return dw_query(s,iid,out,dw_iid_family,dw_iid_font_list);}
static HRes WIN dw_family_collection(void *s,void **out) {(void)s;return dw_out(&dw_collection,out);}
static uint32_t WIN dw_family_count_fonts(DwFamily *f) {return f->count;}
static HRes WIN dw_family_font(DwFamily *f,uint32_t index,void **out) {
    if(out)*out=NULL;
    if(index>=f->count)return E_INVALIDARG_COM;
    return dw_out(f->fonts[index],out);
}
static HRes WIN dw_family_names(DwFamily *f,void **out) {return dw_out(&f->names,out);}
/* Closest font: style first, then stretch, then weight (the CSS font-matching order). */
static HRes WIN dw_family_first_matching(DwFamily *f,uint32_t weight,uint32_t stretch,uint32_t style,void **out) {
    if(out)*out=NULL;
    if(!f->count)return E_INVALIDARG_COM;
    DwFont *best=NULL;long best_score=0;
    for(uint32_t i=0;i<f->count;i++) {
        DwFont *c=f->fonts[i];
        long score=(c->style==style?0:(style && c->style)?1:2)*100000L+labs((long)c->stretch-(long)stretch)*1000L+labs((long)c->weight-(long)weight);
        if(!best || score<best_score){best=c;best_score=score;}
    }
    return dw_out(best,out);
}
DW_STOP(IDWriteFontFamily,GetMatchingFonts)
static void *dw_family_vt[]={dw_family_qi,dw_addref,dw_release,dw_family_collection,dw_family_count_fonts,dw_family_font,
    dw_family_names,dw_family_first_matching,dw_stop_IDWriteFontFamily_GetMatchingFonts};

/* ---- IDWriteFont ---- */
static HRes WIN dw_font_qi(void *s,const void *iid,void **out) {return dw_query(s,iid,out,dw_iid_font,NULL);}
static HRes WIN dw_font_family(DwFont *f,void **out) {return dw_out(f->family,out);}
static uint32_t WIN dw_font_weight(DwFont *f) {return f->weight;}
static uint32_t WIN dw_font_stretch(DwFont *f) {return f->stretch;}
static uint32_t WIN dw_font_style(DwFont *f) {return f->style;}
DW_STOP(IDWriteFont,IsSymbolFont)
static HRes WIN dw_font_face_names(DwFont *f,void **out) {return dw_out(&f->face_names,out);}
DW_STOP(IDWriteFont,GetInformationalStrings)
static uint32_t WIN dw_font_simulations(DwFont *f) {(void)f;return 0;}   /* real faces only: no simulated bold/oblique */
DW_STOP(IDWriteFont,GetMetrics) DW_STOP(IDWriteFont,HasCharacter)
static HRes WIN dw_font_create_face(DwFont *f,void **out) {return dw_out(&f->face,out);}
static void *dw_font_vt[]={dw_font_qi,dw_addref,dw_release,dw_font_family,dw_font_weight,dw_font_stretch,dw_font_style,
    dw_stop_IDWriteFont_IsSymbolFont,dw_font_face_names,dw_stop_IDWriteFont_GetInformationalStrings,dw_font_simulations,
    dw_stop_IDWriteFont_GetMetrics,dw_stop_IDWriteFont_HasCharacter,dw_font_create_face};

/* ---- IDWriteFontFace ---- */
static HRes WIN dw_face_qi(void *s,const void *iid,void **out) {return dw_query(s,iid,out,dw_iid_face,NULL);}
static uint32_t WIN dw_face_type(void *s) {return (uint32_t)DW_OUTER(s,DwFont,face)->type;}   /* CFF 0, TRUETYPE 1, collection 2 */
static HRes WIN dw_face_files(void *s,uint32_t *count,void **files) {
    if(!count)return E_POINTER_COM;
    if(!files){*count=1;return S_OK_;}
    if(*count<1)return E_INVALIDARG_COM;
    *count=1;return dw_out(&DW_OUTER(s,DwFont,face)->file,files);
}
static uint32_t WIN dw_face_index(void *s) {return (uint32_t)DW_OUTER(s,DwFont,face)->index;}
static uint32_t WIN dw_face_simulations(void *s) {(void)s;return 0;}
DW_STOP(IDWriteFontFace,IsSymbolFont) DW_STOP(IDWriteFontFace,GetMetrics) DW_STOP(IDWriteFontFace,GetGlyphCount)
DW_STOP(IDWriteFontFace,GetDesignGlyphMetrics) DW_STOP(IDWriteFontFace,GetGlyphIndices) DW_STOP(IDWriteFontFace,TryGetFontTable)
DW_STOP(IDWriteFontFace,ReleaseFontTable) DW_STOP(IDWriteFontFace,GetGlyphRunOutline) DW_STOP(IDWriteFontFace,GetRecommendedRenderingMode)
DW_STOP(IDWriteFontFace,GetGdiCompatibleMetrics) DW_STOP(IDWriteFontFace,GetGdiCompatibleGlyphMetrics)
static void *dw_face_vt[]={dw_face_qi,dw_addref,dw_release,dw_face_type,dw_face_files,dw_face_index,dw_face_simulations,
    dw_stop_IDWriteFontFace_IsSymbolFont,dw_stop_IDWriteFontFace_GetMetrics,dw_stop_IDWriteFontFace_GetGlyphCount,
    dw_stop_IDWriteFontFace_GetDesignGlyphMetrics,dw_stop_IDWriteFontFace_GetGlyphIndices,dw_stop_IDWriteFontFace_TryGetFontTable,
    dw_stop_IDWriteFontFace_ReleaseFontTable,dw_stop_IDWriteFontFace_GetGlyphRunOutline,
    dw_stop_IDWriteFontFace_GetRecommendedRenderingMode,dw_stop_IDWriteFontFace_GetGdiCompatibleMetrics,
    dw_stop_IDWriteFontFace_GetGdiCompatibleGlyphMetrics};

/* ---- IDWriteFontFile: the reference key is the file path (UTF-16, with its NUL) ---- */
static HRes WIN dw_file_qi(void *s,const void *iid,void **out) {return dw_query(s,iid,out,dw_iid_file,NULL);}
static HRes WIN dw_file_key(void *s,const void **key,uint32_t *size) {
    if(!key || !size)return E_POINTER_COM;
    DwFont *f=DW_OUTER(s,DwFont,file);*key=f->key;*size=f->key_bytes;return S_OK_;
}
static HRes WIN dw_file_loader(void *s,void **out) {(void)s;return dw_out(&dw_loader,out);}
DW_STOP(IDWriteFontFile,Analyze)
static void *dw_file_vt[]={dw_file_qi,dw_addref,dw_release,dw_file_key,dw_file_loader,dw_stop_IDWriteFontFile_Analyze};

/* ---- IDWriteLocalFontFileLoader ---- */
static HRes WIN dw_loader_qi(void *s,const void *iid,void **out) {return dw_query(s,iid,out,dw_iid_local_loader,dw_iid_file_loader);}
DW_STOP(IDWriteLocalFontFileLoader,CreateStreamFromKey)
static uint32_t dw_key_chars(const void *key,uint32_t size) {
    const uint16_t *k=key;uint32_t n=0;while(n<size/2 && k[n])n++;return n;
}
static HRes WIN dw_loader_path_length(void *s,const void *key,uint32_t size,uint32_t *length) {
    (void)s;if(!key || !length)return E_POINTER_COM;
    *length=dw_key_chars(key,size);return S_OK_;
}
static HRes WIN dw_loader_path(void *s,const void *key,uint32_t size,uint16_t *out,uint32_t capacity) {
    (void)s;if(!key || !out)return E_POINTER_COM;
    uint32_t n=dw_key_chars(key,size);
    if(capacity<=n)return DW_E_NOT_SUFFICIENT_BUFFER;
    memcpy(out,key,(size_t)n*2);out[n]=0;return S_OK_;
}
static HRes WIN dw_loader_write_time(void *s,const void *key,uint32_t size,uint64_t *out) {
    (void)s;if(!key || !out)return E_POINTER_COM;
    char path[4096];uint16_t wide[4096];uint32_t n=dw_key_chars(key,size);
    if(n>=4096)return E_INVALIDARG_COM;
    memcpy(wide,key,(size_t)n*2);wide[n]=0;
    struct stat st;
    if(wide_to_multi_byte(65001,0,wide,-1,path,sizeof path,NULL,NULL)<=0 || stat(path,&st))return (HRes)0x80070002;   /* file not found */
    *out=((uint64_t)st.st_mtim.tv_sec+11644473600ULL)*10000000+(uint64_t)st.st_mtim.tv_nsec/100;return S_OK_;
}
static void *dw_loader_vt[]={dw_loader_qi,dw_addref,dw_release,dw_stop_IDWriteLocalFontFileLoader_CreateStreamFromKey,
    dw_loader_path_length,dw_loader_path,dw_loader_write_time};

/* ---- IDWriteLocalizedStrings: one entry, locale en-us ---- */
static HRes WIN dw_strings_qi(void *s,const void *iid,void **out) {return dw_query(s,iid,out,dw_iid_strings,NULL);}
static uint32_t WIN dw_strings_count(void *s) {(void)s;return 1;}
static HRes WIN dw_strings_find(void *s,const uint16_t *locale,uint32_t *index,int32_t *exists) {
    (void)s;if(!locale || !index || !exists)return E_POINTER_COM;
    static const char en[]="en-us";int same=1;
    for(int i=0;i<6;i++){uint16_t c=locale[i];if(c>='A' && c<='Z')c+=32;if(c!=(uint16_t)(unsigned char)en[i]){same=0;break;}}
    *index=same?0:UINT32_MAX;*exists=same;return S_OK_;
}
static HRes dw_copy_text(const char *text,uint16_t *out,uint32_t capacity) {
    int n=multi_byte_to_wide(65001,0,text,-1,NULL,0);
    if(n<=0)return E_INVALIDARG_COM;
    if(capacity<(uint32_t)n)return DW_E_NOT_SUFFICIENT_BUFFER;
    multi_byte_to_wide(65001,0,text,-1,out,n);return S_OK_;
}
static HRes WIN dw_strings_locale_length(void *s,uint32_t index,uint32_t *length) {
    (void)s;if(!length)return E_POINTER_COM;
    if(index)return E_INVALIDARG_COM;
    *length=5;return S_OK_;
}
static HRes WIN dw_strings_locale(void *s,uint32_t index,uint16_t *out,uint32_t capacity) {
    (void)s;if(!out)return E_POINTER_COM;
    if(index)return E_INVALIDARG_COM;
    return dw_copy_text("en-us",out,capacity);
}
static HRes WIN dw_strings_length(DwStrings *s,uint32_t index,uint32_t *length) {
    if(!length)return E_POINTER_COM;
    if(index)return E_INVALIDARG_COM;
    int n=multi_byte_to_wide(65001,0,s->text,-1,NULL,0);*length=n>0?(uint32_t)n-1:0;return S_OK_;
}
static HRes WIN dw_strings_get(DwStrings *s,uint32_t index,uint16_t *out,uint32_t capacity) {
    if(!out)return E_POINTER_COM;
    if(index)return E_INVALIDARG_COM;
    return dw_copy_text(s->text,out,capacity);
}
static void *dw_strings_vt[]={dw_strings_qi,dw_addref,dw_release,dw_strings_count,dw_strings_find,dw_strings_locale_length,
    dw_strings_locale,dw_strings_length,dw_strings_get};

/* ---- IDWriteFontFallback: character coverage from fontconfig ---- */
static FcCharSet *dw_font_charset(DwFont *f) {
    pthread_mutex_lock(&dw_mutex);
    if(!f->charset_loaded) {
        f->charset_loaded=1;
        FcPattern *p=FcPatternBuild(NULL,FC_FILE,FcTypeString,f->path,FC_INDEX,FcTypeInteger,f->index,(char *)NULL);
        FcObjectSet *os=FcObjectSetBuild(FC_CHARSET,(char *)NULL);
        FcFontSet *set=p && os?FcFontList(NULL,p,os):NULL;
        FcCharSet *cs;
        if(set && set->nfont && FcPatternGetCharSet(set->fonts[0],FC_CHARSET,0,&cs)==FcResultMatch)f->charset=FcCharSetCopy(cs);
        if(set)FcFontSetDestroy(set);
        if(os)FcObjectSetDestroy(os);
        if(p)FcPatternDestroy(p);
    }
    pthread_mutex_unlock(&dw_mutex);
    return f->charset;
}
static int dw_covers(DwFont *f,uint32_t c) {FcCharSet *cs=dw_font_charset(f);return cs && FcCharSetHasChar(cs,c);}
static DwFont *dw_font_for_file(const char *path,int index) {
    for(uint32_t i=0;i<dw_family_count;i++)for(uint32_t j=0;j<dw_families[i]->count;j++){
        DwFont *f=dw_families[i]->fonts[j];if(f->index==index && !strcmp(f->path,path))return f;
    }
    return NULL;
}
static uint32_t dw_next_char(const uint16_t *t,uint32_t n,uint32_t *units) {
    if(n>=2 && t[0]>=0xD800 && t[0]<0xDC00 && t[1]>=0xDC00 && t[1]<0xE000){*units=2;return 0x10000+((uint32_t)(t[0]-0xD800)<<10)+(t[1]-0xDC00);}
    *units=1;return t[0];
}
typedef HRes (WIN *DwGetTextAtPosition)(void *,uint32_t,const uint16_t **,uint32_t *);
/* The requested family's best font when it covers the first character, otherwise fontconfig's
 * best match covering it; the mapped length runs while that font covers the text. A character no
 * system font covers maps to no font (its length is still reported), as DirectWrite does. */
static HRes WIN dw_map_characters(void *self,void *source,uint32_t position,uint32_t length,void *base_collection,
                                  const uint16_t *base_family,uint32_t weight,uint32_t style,uint32_t stretch,
                                  uint32_t *mapped_length,void **mapped_font,float *scale) {
    (void)self;(void)base_collection;
    if(!source || !mapped_length || !mapped_font || !scale)return E_INVALIDARG_COM;
    *mapped_length=0;*mapped_font=NULL;*scale=1.0f;
    const uint16_t *text=NULL;uint32_t available=0;
    HRes r=((DwGetTextAtPosition)(*(void ***)source)[3])(source,position,&text,&available);
    if(r<0)return r;
    uint32_t n=available<length?available:length;
    if(!n || !text)return S_OK_;
    uint32_t units;uint32_t first=dw_next_char(text,n,&units);
    DwFont *font=NULL;
    char family_name[512]="";
    if(base_family && *base_family)wide_to_multi_byte(65001,0,base_family,-1,family_name,sizeof family_name,NULL,NULL);
    for(uint32_t i=0;family_name[0] && i<dw_family_count && !font;i++)
        if(!strcasecmp(dw_families[i]->name,family_name)) {
            DwFont *candidate=NULL;
            if(dw_family_first_matching(dw_families[i],weight,stretch,style,(void **)&candidate)==S_OK_ && candidate) {
                if(dw_covers(candidate,first))font=candidate;
                dw_release((DwObject *)candidate);
            }
        }
    if(!font) {
        FcPattern *p=FcPatternCreate();FcCharSet *cs=FcCharSetCreate();
        if(p && cs) {
            FcCharSetAddChar(cs,first);FcPatternAddCharSet(p,FC_CHARSET,cs);
            FcPatternAddInteger(p,FC_WEIGHT,FcWeightFromOpenType((int)weight));
            FcPatternAddInteger(p,FC_SLANT,style==2?FC_SLANT_ITALIC:style==1?FC_SLANT_OBLIQUE:FC_SLANT_ROMAN);
            if(family_name[0])FcPatternAddString(p,FC_FAMILY,(const FcChar8 *)family_name);
            FcConfigSubstitute(NULL,p,FcMatchPattern);FcDefaultSubstitute(p);
            FcResult result;FcPattern *match=FcFontMatch(NULL,p,&result);
            FcChar8 *file;int index=0;
            if(match && FcPatternGetString(match,FC_FILE,0,&file)==FcResultMatch) {
                FcPatternGetInteger(match,FC_INDEX,0,&index);
                DwFont *candidate=dw_font_for_file((const char *)file,index);
                if(candidate && dw_covers(candidate,first))font=candidate;
            }
            if(match)FcPatternDestroy(match);
        }
        if(cs)FcCharSetDestroy(cs);
        if(p)FcPatternDestroy(p);
    }
    uint32_t used=units;
    while(font && used<n){uint32_t u;uint32_t c=dw_next_char(text+used,n-used,&u);if(!dw_covers(font,c))break;used+=u;}
    *mapped_length=used;
    if(font){dw_addref((DwObject *)font);*mapped_font=font;}
    return S_OK_;
}
static HRes WIN dw_fallback_qi(void *s,const void *iid,void **out) {return dw_query(s,iid,out,dw_iid_fallback,NULL);}
static void *dw_fallback_vt[]={dw_fallback_qi,dw_addref,dw_release,dw_map_characters};

/* DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED 0 / ISOLATED 1, IID_IDWriteFactory, out).
 * IDWriteFactory1/2 are the same object; IDWriteFactory3..7 are not provided: E_NOINTERFACE. */
HRes WIN peekgame_dwrite_create_factory(uint32_t type,const void *iid,void **factory) {
    if(!factory)return E_POINTER_COM;
    *factory=NULL;
    if(type>1)return E_INVALIDARG_COM;
    if(!dw_iid_is(iid,dw_iid_factory) && !dw_iid_is(iid,dw_iid_factory1) && !dw_iid_is(iid,dw_iid_factory2) &&
       !dw_iid_is(iid,dw_iid_unknown))return E_NOINTERFACE_COM;
    pthread_mutex_lock(&dw_mutex);int ok=dw_build();pthread_mutex_unlock(&dw_mutex);
    if(!ok)return (HRes)0x80004005;                                   /* E_FAIL: no font list */
    if(type==1) {
        DwObject *isolated=calloc(1,sizeof *isolated);
        if(!isolated)return E_OUTOFMEMORY_COM;
        isolated->vt=dw_factory_vt;isolated->refs=1;*factory=isolated;return S_OK_;
    }
    return dw_out(&dw_factory,factory);
}
