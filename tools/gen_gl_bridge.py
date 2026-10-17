"""Generate typed Win64->SysV GL thunks from host Khronos headers (no SDL)."""
from pathlib import Path
import re,sys
out=Path(sys.argv[1]); functions={};core=set()
for path in [Path('/usr/include/GL/gl.h'),Path('/usr/include/GL/glext.h')]:
 text=path.read_text()
 for m in re.finditer(r'^GLAPI\s+(.*?);',text,re.M|re.S):
  decl=' '.join(m.group(1).split())
  match=re.match(r'(.+?)\s*(?:GLAPIENTRY|APIENTRY)\s*(gl\w+)\s*\((.*)\)$',decl)
  if not match:continue
  ret,name,params=match.groups()
  if path.name=='gl.h':core.add(name)
  if 'PROC' in ret or re.search(r'\bGL\w*PROC\w*\b',params):continue
  functions.setdefault(name,(ret,params))
names=sorted(functions)
lines=['/* Generated typed OpenGL ABI adapters. */','#include <GL/gl.h>','#include <GL/glext.h>','#include <GL/glx.h>',
 '#include <stdatomic.h>','#include <stdio.h>','#include <stdlib.h>','#include <string.h>',
 '#define WIN __attribute__((ms_abi))',f'static _Atomic(void*) targets[{len(names)}];',
 'static void *resolve(unsigned i,const char *name) {',
 ' void *p=atomic_load_explicit(&targets[i],memory_order_acquire);',
 ' if(!p){p=(void*)glXGetProcAddressARB((const unsigned char*)name);',
 ' if(!p){fprintf(stderr,"STOP: host OpenGL function unavailable: %s\\n",name);_Exit(78);}',
 ' atomic_store_explicit(&targets[i],p,memory_order_release);}return p;}']
declarations=[];exports=[]
for i,name in enumerate(names):
 ret,params=functions[name];args=[]
 if params.strip() not in ('','void'):
  for arg in params.split(','):args.append(re.findall(r'(\w+)\s*$',re.sub(r'\[.*?\]','',arg).strip())[0])
 params=params or 'void'
 call=f'(({ret}(*)({params}))resolve({i},"{name}"))({",".join(args)})'
 lines.append(f'{ret} WIN peekgame_gl_{i}({params}) {{'+('' if ret=='void' else 'return ')+call+';}')
 if name in core:
  declarations.append(f'pub fn peekgame_gl_{i}();')
  exports.append(f'export!("opengl32.dll","{name}","graphics/opengl","partial","typed host OpenGL adapter",crate::api::graphics::opengl::thunks::peekgame_gl_{i}),')
lines+=['static const char *names[]={'+','.join('"'+n+'"' for n in names)+'};',
 'static void *wrappers[]={'+','.join(f'(void*)peekgame_gl_{i}' for i in range(len(names)))+'};',
 'void *peekgame_gl_lookup(const char *name){',
 ' size_t lo=0,hi=sizeof(names)/sizeof(names[0]);while(lo<hi){size_t mid=lo+(hi-lo)/2;int c=strcmp(name,names[mid]);',
 ' if(c<0)hi=mid;else if(c>0)lo=mid+1;else {if(!glXGetProcAddressARB((const unsigned char*)name))return NULL;return wrappers[mid];}}return NULL;}']
(out/'gl_thunks.c').write_text('\n'.join(lines))
(out/'gl_ffi.rs').write_text('unsafe extern "win64" {\n'+'\n'.join(declarations)+'\n}')
(out/'gl_exports.rs').write_text('exports! {\n'+'\n'.join(exports)+'\n}')
print(f'OpenGL bridge: {len(names)} typed thunks; {len(exports)} static GL exports')
