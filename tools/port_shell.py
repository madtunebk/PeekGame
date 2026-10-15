from pathlib import Path
files={
'src/host/arguments.rs':r'''pub fn split(input:&[u16])->Vec<Vec<u16>>{
 let mut result=Vec::new();let mut token=Vec::new();let mut quoted=false;let mut slashes=0;let mut started=false;
 for &c in input {
  if c==92{slashes+=1;started=true;continue;}
  if c==34{token.extend(std::iter::repeat_n(92,slashes/2));if slashes%2==1{token.push(34);}else{quoted=!quoted;}slashes=0;started=true;continue;}
  token.extend(std::iter::repeat_n(92,slashes));slashes=0;
  if (c==32||c==9)&&!quoted{if started{result.push(std::mem::take(&mut token));started=false;}}
  else{token.push(c);started=true;}
 }
 token.extend(std::iter::repeat_n(92,slashes));if started{result.push(token);}result
}
''',
'src/api/memory/mod.rs':'pub(crate) mod local;\n',
'src/api/memory/local.rs':r'''use std::{collections::HashMap,ffi::c_void,sync::{Mutex,OnceLock}};
unsafe extern "C"{fn malloc(n:usize)->*mut c_void;fn calloc(n:usize,size:usize)->*mut c_void;fn free(p:*mut c_void);}
static ALLOCATIONS:OnceLock<Mutex<HashMap<usize,usize>>>=OnceLock::new();
fn allocations()-> &'static Mutex<HashMap<usize,usize>>{ALLOCATIONS.get_or_init(||Mutex::new(HashMap::new()))}
pub extern "win64" fn allocate(flags:u32,size:usize)->usize{
 if flags&!0x40!=0{crate::api::system::error::set(50);return 0;}
 let p=unsafe{if flags&0x40!=0{calloc(1,size.max(1))}else{malloc(size.max(1))}};
 if p.is_null(){crate::api::system::error::set(8);return 0;}
 allocations().lock().unwrap().insert(p as usize,size);p as usize
}
pub extern "win64" fn release(handle:usize)->usize{
 if handle==0{return 0;}if allocations().lock().unwrap().remove(&handle).is_none(){crate::api::system::error::set(6);return handle;}
 unsafe{free(handle as *mut c_void);}0
}
pub extern "win64" fn size(handle:usize)->usize{match allocations().lock().unwrap().get(&handle){Some(size)=>*size,None=>{crate::api::system::error::set(6);0}}}
''',
'src/api/shell/mod.rs':'pub(crate) mod arguments;\n',
'src/api/shell/arguments.rs':r'''use crate::{api::{system::error,memory::local},host::arguments};
pub unsafe extern "win64" fn command_line_to_argv(input:*const u16,count:*mut i32)->*mut *mut u16{
 if input.is_null()||count.is_null(){error::set(87);return std::ptr::null_mut();}
 let mut n=0;while unsafe{*input.add(n)}!=0{n+=1;}
 let input=unsafe{std::slice::from_raw_parts(input,n)};
 let mut args=arguments::split(input);
 if args.is_empty(){if let Some(process)=crate::host::process::PROCESS.get(){let line=process.command_line.to_string_lossy().encode_utf16().collect::<Vec<_>>();args=arguments::split(&line).into_iter().take(1).collect();}}
 else if input.first().is_some_and(|c|*c==32||*c==9){args.insert(0,Vec::new());}
 let header=(args.len()+1)*8;let bytes=header+args.iter().map(|s|(s.len()+1)*2).sum::<usize>();
 let base=local::allocate(0x40,bytes) as *mut u8;if base.is_null(){return std::ptr::null_mut();}
 let pointers=base.cast::<*mut u16>();let mut offset=header;
 for (i,arg) in args.iter().enumerate(){let text=unsafe{base.add(offset).cast::<u16>()};unsafe{*pointers.add(i)=text;std::ptr::copy_nonoverlapping(arg.as_ptr(),text,arg.len());}offset+=(arg.len()+1)*2;}
 unsafe{*count=args.len() as i32;}pointers
}
''',
'src/registry/shell.rs':r'''use super::*;
use crate::api::{shell::arguments,memory::local};
pub(super) static EXPORTS:&[Export]=&[
 Export{dll:"shell32.dll",name:"CommandLineToArgvW",category:"shell",status:"partial",scope:"UTF-16 argument parsing; LocalFree owns returned block",address:Some(||arguments::command_line_to_argv as *const std::ffi::c_void)},
 implemented!("LocalAlloc","memory","partial","fixed allocation with optional zero-fill; movable unsupported",local::allocate),
 implemented!("LocalFree","memory","partial","registered LocalAlloc blocks",local::release),
 implemented!("LocalSize","memory","partial","registered LocalAlloc sizes",local::size),
];
'''
}
for n,s in files.items():p=Path(n);p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s)
p=Path('src/api/mod.rs');p.write_text(p.read_text()+'pub(crate) mod shell;\npub(crate) mod memory;\n')
p=Path('src/host/mod.rs');p.write_text(p.read_text()+'pub(crate) mod arguments;\n')
p=Path('src/registry/mod.rs');s=p.read_text().replace('mod graphics;','mod graphics;\nmod shell;').replace('.chain(crt::EXPORTS.iter())','.chain(shell::EXPORTS.iter())\n        .chain(crt::EXPORTS.iter())');p.write_text(s)
