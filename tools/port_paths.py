from pathlib import Path
files={
'src/host/paths.rs':r'''use std::{ffi::{CStr,c_char},path::{Path,PathBuf}};
pub unsafe fn narrow(p:*const c_char)->Result<String,u32>{if p.is_null(){return Err(87);}unsafe{CStr::from_ptr(p)}.to_str().map(str::to_owned).map_err(|_|1113)}
pub unsafe fn wide(p:*const u16)->Result<String,u32>{if p.is_null(){return Err(87);}let mut len=0;while unsafe{*p.add(len)}!=0{len+=1;}String::from_utf16(unsafe{std::slice::from_raw_parts(p,len)}).map_err(|_|1113)}
pub fn host(text:&str)->Result<PathBuf,u32>{
 let mut text=text.replace('\\',"/");if text.starts_with("//?/"){text=text[4..].to_owned();}
 if text.as_bytes().get(1)==Some(&b':'){if !text[..1].eq_ignore_ascii_case("z"){return Err(3);}text=text[2..].to_owned();}
 if text.is_empty(){return Err(3);}Ok(PathBuf::from(text))
}
pub fn windows(path:&Path)->String{let text=path.to_string_lossy().replace('/',"\\");if path.is_absolute(){format!("Z:{text}")}else{text}}
pub fn error(e:&std::io::Error)->u32{match e.raw_os_error(){Some(2)=>2,Some(20)=>3,Some(13|1)=>5,Some(17)=>183,Some(39)=>145,Some(28)=>112,_=>50}}
pub fn full(path:&str)->Result<PathBuf,u32>{let path=host(path)?;let path=if path.is_absolute(){path}else{std::env::current_dir().map_err(|e|error(&e))?.join(path)};let mut result=PathBuf::new();for component in path.components(){match component{std::path::Component::ParentDir=>{result.pop();},std::path::Component::CurDir=>{},value=>result.push(value.as_os_str())}}Ok(result)}
''',
'src/api/filesystem/paths.rs':r'''use crate::{api::system::error,host::paths};
use std::{ffi::c_char,path::PathBuf};
fn fail(code:u32)->u32{error::set(code);0}
unsafe fn output(text:&str,out:*mut u16,size:u32)->u32{
 let text=text.encode_utf16().collect::<Vec<_>>();let needed=text.len()+1;
 if needed>size as usize{return needed as u32;}if out.is_null(){return fail(87);}
 unsafe{std::ptr::copy_nonoverlapping(text.as_ptr(),out,text.len());*out.add(text.len())=0;}text.len() as u32
}
pub unsafe extern "win64" fn current_w(size:u32,out:*mut u16)->u32{match std::env::current_dir(){Ok(path)=>unsafe{output(&paths::windows(&path),out,size)},Err(e)=>fail(paths::error(&e))}}
pub unsafe extern "win64" fn set_current_w(input:*const u16)->i32{let path=unsafe{paths::wide(input)}.and_then(|s|paths::host(&s));match path{Ok(path)=>match std::env::set_current_dir(path){Ok(())=>1,Err(e)=>{fail(paths::error(&e));0}},Err(e)=>{fail(e);0}}}
pub unsafe extern "win64" fn full_w(input:*const u16,size:u32,out:*mut u16,part:*mut *mut u16)->u32{
 if !part.is_null(){unsafe{*part=std::ptr::null_mut();}}
 let path=unsafe{paths::wide(input)}.and_then(|s|paths::full(&s));match path{Ok(path)=>{let text=paths::windows(&path);let result=unsafe{output(&text,out,size)};if result>0&&result<size&&!part.is_null(){let offset=text.rfind('\\').map_or(0,|i|text[..i+1].encode_utf16().count());unsafe{*part=out.add(offset);}}result},Err(e)=>fail(e)}
}
pub unsafe extern "win64" fn module_w(module:usize,out:*mut u16,size:u32)->u32{
 let Some(process)=crate::host::process::PROCESS.get()else{return fail(5023);};
 if module!=0&&module!=process.image_base{return fail(126);}if size==0{return fail(122);}
 let line=process.command_line.to_string_lossy().encode_utf16().collect::<Vec<_>>();let args=crate::host::arguments::split(&line);
 let Some(name)=args.first()else{return fail(87);};let path=PathBuf::from(String::from_utf16_lossy(name));let text=paths::windows(&path).encode_utf16().collect::<Vec<_>>();
 if out.is_null(){return fail(87);}let n=text.len().min(size as usize-1);unsafe{std::ptr::copy_nonoverlapping(text.as_ptr(),out,n);*out.add(n)=0;}
 if text.len()>=size as usize{error::set(122);size}else{text.len() as u32}
}
pub unsafe extern "win64" fn attributes_w(input:*const u16)->u32{match unsafe{paths::wide(input)}.and_then(|s|paths::host(&s)){Ok(path)=>match std::fs::metadata(path){Ok(m)=>{let mut value=if m.is_dir(){0x10}else{0x80};if m.permissions().readonly(){value|=1;}value},Err(e)=>{error::set(paths::error(&e));u32::MAX}},Err(e)=>{error::set(e);u32::MAX}}}
pub unsafe extern "win64" fn attributes_a(input:*const c_char)->u32{match unsafe{paths::narrow(input)}{Ok(s)=>{let w=s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();unsafe{attributes_w(w.as_ptr())}},Err(e)=>{error::set(e);u32::MAX}}}
''',
'src/registry/filesystem.rs':r'''use super::*;
use crate::api::filesystem::paths;
pub(super) static EXPORTS:&[Export]=&[
 implemented!("GetCurrentDirectoryW","filesystem","partial","host current directory through Z: root mapping",paths::current_w),
 implemented!("SetCurrentDirectoryW","filesystem","partial","host current directory through Z: root mapping",paths::set_current_w),
 implemented!("GetFullPathNameW","filesystem","partial","lexical absolute path through Z: mapping",paths::full_w),
 implemented!("GetModuleFileNameW","filesystem","partial","configured main executable; runtime module files unavailable",paths::module_w),
 implemented!("GetFileAttributesW","filesystem","partial","host file/directory/read-only attributes",paths::attributes_w),
 implemented!("GetFileAttributesA","filesystem","partial","host file/directory/read-only attributes; UTF-8",paths::attributes_a),
];
'''
}
for n,s in files.items():Path(n).write_text(s)
p=Path('src/api/filesystem/mod.rs');p.write_text(p.read_text()+'pub(crate) mod paths;\n')
p=Path('src/host/mod.rs');p.write_text(p.read_text()+'pub(crate) mod paths;\n')
p=Path('src/registry/mod.rs');s=p.read_text().replace('mod graphics;','mod graphics;\nmod filesystem;').replace('.chain(shell::EXPORTS.iter())','.chain(filesystem::EXPORTS.iter())\n        .chain(shell::EXPORTS.iter())');p.write_text(s)
