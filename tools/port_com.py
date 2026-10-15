from pathlib import Path
files={
'src/api/com/mod.rs':'pub(crate) mod apartment;\n',
'src/api/com/apartment.rs':r'''use std::{cell::RefCell,ffi::c_void};
thread_local!{static APARTMENT:RefCell<Option<(u32,u32)>>=const{RefCell::new(None)};}
pub extern "win64" fn initialize(reserved:*mut c_void,flags:u32)->i32{
 if !reserved.is_null()||flags&!14!=0{return 0x80070057u32 as i32;}
 let model=flags&2;APARTMENT.with(|state|{let mut state=state.borrow_mut();match state.as_mut(){None=>{*state=Some((model,1));0},Some((current,count)) if *current==model=>{*count+=1;1},_=>0x80010106u32 as i32}})
}
pub extern "win64" fn uninitialize(){APARTMENT.with(|state|{let mut state=state.borrow_mut();if let Some((_,count))=state.as_mut(){*count-=1;if *count==0{*state=None;}}});}
pub extern "win64" fn ole_initialize(reserved:*mut c_void)->i32{initialize(reserved,2)}
pub unsafe extern "win64" fn create_instance(_class:*const u8,_outer:*mut c_void,_context:u32,_iid:*const u8,out:*mut *mut c_void)->i32{
 if out.is_null(){return 0x80004003u32 as i32;}unsafe{*out=std::ptr::null_mut();}
 let initialized=APARTMENT.with(|state|state.borrow().is_some());
 if !initialized{return 0x800401f0u32 as i32;}0x80040154u32 as i32
}
unsafe extern "C"{fn malloc(size:usize)->*mut c_void;fn realloc(p:*mut c_void,size:usize)->*mut c_void;fn free(p:*mut c_void);}
pub extern "win64" fn allocate(size:usize)->*mut c_void{unsafe{malloc(size.max(1))}}
pub unsafe extern "win64" fn reallocate(p:*mut c_void,size:usize)->*mut c_void{unsafe{realloc(p,size)}}
pub unsafe extern "win64" fn release(p:*mut c_void){unsafe{free(p);}}
''',
'src/registry/com.rs':r'''use super::Export;
use crate::api::com::apartment;
macro_rules! com{($name:literal,$function:path)=>{Export{dll:"ole32.dll",name:$name,category:"com",status:"partial",scope:"logical COM apartments and task allocator; no registered COM classes",address:Some(||$function as *const std::ffi::c_void)}};}
pub(super) static EXPORTS:&[Export]=&[
 com!("CoInitializeEx",apartment::initialize),com!("CoUninitialize",apartment::uninitialize),com!("OleInitialize",apartment::ole_initialize),com!("OleUninitialize",apartment::uninitialize),com!("CoCreateInstance",apartment::create_instance),com!("CoTaskMemAlloc",apartment::allocate),com!("CoTaskMemRealloc",apartment::reallocate),com!("CoTaskMemFree",apartment::release),
];
'''
}
for n,s in files.items():p=Path(n);p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s)
p=Path('src/api/mod.rs');p.write_text(p.read_text()+'pub(crate) mod com;\n')
p=Path('src/registry/mod.rs');s=p.read_text().replace('mod graphics;','mod graphics;\nmod com;').replace('.chain(filesystem::EXPORTS.iter())','.chain(com::EXPORTS.iter())\n        .chain(filesystem::EXPORTS.iter())');p.write_text(s)
