from pathlib import Path
files={
'src/loader/traps.rs':r'''use std::ffi::c_void;
unsafe extern "C"{fn mmap(a:*mut c_void,n:usize,p:i32,f:i32,fd:i32,o:i64)->*mut c_void;fn mprotect(a:*mut c_void,n:usize,p:i32)->i32;fn munmap(a:*mut c_void,n:usize)->i32;}
pub struct Traps{memory:*mut u8,size:usize,names:Vec<Box<str>>}
impl Traps {
 pub fn new(names:Vec<String>)->Result<Self,String>{
  let size=(names.len().max(1)*32).div_ceil(4096)*4096;
  let memory=unsafe{mmap(std::ptr::null_mut(),size,3,0x22,-1,0)};
  if memory as isize == -1{return Err(std::io::Error::last_os_error().to_string());}
  let result=Self{memory:memory.cast(),size,names:names.into_iter().map(String::into_boxed_str).collect()};
  for (i,name) in result.names.iter().enumerate(){
   // Windows ABI, original return address remains on stack. Never returns.
   let mut code=vec![0x48,0xb9];code.extend_from_slice(&(name.as_ref() as *const str as *const u8 as usize).to_le_bytes());
   code.extend_from_slice(&[0x48,0xba]);code.extend_from_slice(&name.len().to_le_bytes());
   code.extend_from_slice(&[0x4c,0x8b,0x04,0x24]);
   code.extend_from_slice(&[0x48,0xb8]);code.extend_from_slice(&(missing as *const () as usize).to_le_bytes());
   code.extend_from_slice(&[0xff,0xe0]);
   unsafe{std::ptr::copy_nonoverlapping(code.as_ptr(),result.memory.add(i*40),code.len());}
  }
  if unsafe{mprotect(memory,size,5)}!=0{return Err(std::io::Error::last_os_error().to_string());}Ok(result)
 }
 pub fn address(&self,index:usize)->usize{unsafe{self.memory.add(index*40) as usize}}
}
impl Drop for Traps{fn drop(&mut self){unsafe{munmap(self.memory.cast(),self.size);}}}
extern "win64" fn missing(name:*const u8,len:usize,caller:usize)->!{
 let name=unsafe{std::str::from_utf8_unchecked(std::slice::from_raw_parts(name,len))};
 eprintln!("STOP: missing Windows API {name}, called from {caller:#x}");std::process::exit(78)
}
''',
'src/host/teb.rs':r'''use std::{cell::RefCell,ffi::c_void};
unsafe extern "C" {
 fn syscall(number:i64,...)->i64;
 fn pthread_self()->usize;
 fn pthread_getattr_np(thread:usize,attr:*mut u8)->i32;
 fn pthread_attr_getstack(attr:*const u8,stack:*mut *mut c_void,size:*mut usize)->i32;
 fn pthread_attr_destroy(attr:*mut u8)->i32;
 fn getpid()->i32;
}
struct ThreadEnvironment{teb:Box<[u64;1024]>,_peb:Box<[u64;128]>,_vector:Box<[usize;64]>,_data:Vec<u8>,previous:u64}
thread_local!{static ENVIRONMENT:RefCell<Option<ThreadEnvironment>>=const{RefCell::new(None)};}
pub fn initialize(image:usize,data:Vec<u8>)->Result<(),String>{
 ENVIRONMENT.with(|cell|{
  if cell.borrow().is_some(){return Err("thread environment already initialized".into());}
  let mut environment=ThreadEnvironment{teb:Box::new([0;1024]),_peb:Box::new([0;128]),_vector:Box::new([0;64]),_data:data,previous:0};
  let mut attr=[0u64;8];let mut stack=std::ptr::null_mut();let mut size=0usize;
  unsafe{
   if pthread_getattr_np(pthread_self(),attr.as_mut_ptr().cast())!=0{return Err("cannot query Windows thread stack".into());}
   let code=pthread_attr_getstack(attr.as_ptr().cast(),&mut stack,&mut size);pthread_attr_destroy(attr.as_mut_ptr().cast());
   if code!=0{return Err("cannot query stack bounds".into());}
   environment.teb[1]=stack as u64+size as u64;environment.teb[2]=stack as u64;
   environment.teb[6]=environment.teb.as_ptr() as u64;
   environment.teb[8]=getpid() as u64;environment.teb[9]=syscall(186) as u64;
   environment._peb[2]=image as u64;
   environment.teb[12]=environment._peb.as_ptr() as u64;
   environment._vector[0]=environment._data.as_ptr() as usize;
   environment.teb[11]=environment._vector.as_ptr() as u64;
   if syscall(158,0x1004i32,&mut environment.previous)!=0{return Err("ARCH_GET_GS failed".into());}
   if syscall(158,0x1001i32,environment.teb.as_ptr())!=0{return Err("ARCH_SET_GS failed".into());}
  }
  *cell.borrow_mut()=Some(environment);Ok(())
 })
}
pub fn set_error(value:u32){ENVIRONMENT.with(|env|{if let Some(env)=env.borrow_mut().as_mut(){unsafe{env.teb.as_mut_ptr().cast::<u8>().add(0x68).cast::<u32>().write(value);}}});}
pub fn error()->Option<u32>{ENVIRONMENT.with(|env|env.borrow().as_ref().map(|env|unsafe{env.teb.as_ptr().cast::<u8>().add(0x68).cast::<u32>().read()}))}
impl Drop for ThreadEnvironment{fn drop(&mut self){unsafe{syscall(158,0x1001i32,self.previous);}}}
''',
'src/loader/tls.rs':r'''use super::mapping::Mapping;
pub struct Tls {pub data:Vec<u8>,callbacks:Vec<usize>}
impl Tls {
 pub fn prepare(mapped:&Mapping,bytes:&[u8])->Result<Self,String>{
  let pe=u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;let opt=pe+24;
  let dirs=u32::from_le_bytes(bytes[opt+108..opt+112].try_into().unwrap());
  let mut result=Self{data:Vec::new(),callbacks:Vec::new()};
  if dirs<=9{return Ok(result);}
  let rva=u32::from_le_bytes(bytes[opt+184..opt+188].try_into().unwrap()) as usize;
  if rva==0{return Ok(result);}
  let tls=mapped.read(mapped.base()+rva,40)?;
  let q=|offset|u64::from_le_bytes(tls[offset..offset+8].try_into().unwrap()) as usize;
  let (start,end,index,callbacks)=(q(0),q(8),q(16),q(24));
  let zero=u32::from_le_bytes(tls[32..36].try_into().unwrap()) as usize;
  let size=end.checked_sub(start).ok_or("invalid TLS template range")?;
  let total=size.checked_add(zero).filter(|n|*n<=16*1024*1024).ok_or("TLS template too large")?;
  result.data=vec![0;total.max(1)];
  if size>0{result.data[..size].copy_from_slice(mapped.read(start,size)?);}
  mapped.write_u32(index,0)?;
  if callbacks!=0 {
   for i in 0..64 {
    let bytes=mapped.read(callbacks+i*8,8)?;let address=u64::from_le_bytes(bytes.try_into().unwrap()) as usize;
    if address==0{return Ok(result);}
    if !mapped.executable(address){return Err("TLS callback outside executable section".into());}
    result.callbacks.push(address);
   }
   return Err("unterminated TLS callback array".into());
  }
  Ok(result)
 }
 pub fn attach(&self,base:usize){for &address in &self.callbacks{eprintln!("TLS callback {address:#x}: DLL_PROCESS_ATTACH");let callback:extern "win64" fn(usize,u32,usize)=unsafe{std::mem::transmute(address)};callback(base,1,0);}}
}
'''
}
for name,text in files.items():
 p=Path(name);p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
p=Path('src/loader/traps.rs');p.write_text(p.read_text().replace('names.len().max(1)*32','names.len().max(1)*40'))
p=Path('src/loader/mapping.rs');s=p.read_text();a=s.index('        let directories =');b=s.index('        // MAP_FIXED',a);s=s[:a]+s[b:];s=s.replace('    pub fn base(&self)',r'''    pub fn read(&self,address:usize,n:usize)->Result<&[u8],String>{
        let offset=address.checked_sub(self.base()).ok_or("VA below image")?;
        if offset.checked_add(n).is_none_or(|end|end>self.size){return Err("VA outside image".into());}
        Ok(unsafe{std::slice::from_raw_parts(self.address.add(offset),n)})
    }
    pub fn write_u32(&self,address:usize,value:u32)->Result<(),String>{self.read(address,4)?;unsafe{(address as *mut u32).write_unaligned(value);}Ok(())}
    pub fn executable(&self,address:usize)->bool{address.checked_sub(self.base()).is_some_and(|offset|self.sections.iter().any(|s|s.protection&4!=0&&offset>=s.start&&offset<s.start+s.size))}
    pub fn base(&self)''');p.write_text(s)
p=Path('src/host/mod.rs');p.write_text(p.read_text()+'\npub(crate) mod teb;\n')
p=Path('src/lib.rs');p.write_text(p.read_text()+'\npub use host::teb::initialize as initialize_windows_thread;\n')
p=Path('src/api/system/error.rs');s=p.read_text().replace('    LAST_ERROR.set(code);','    LAST_ERROR.set(code);\n    crate::host::teb::set_error(code);').replace('    LAST_ERROR.get()','    crate::host::teb::error().unwrap_or_else(||LAST_ERROR.get())');p.write_text(s)
p=Path('src/loader/mod.rs');s=p.read_text().replace('mod mapping;','mod mapping;\nmod tls;\nmod traps;');s=s.replace('    let mut unresolved = Vec::new();','    let tls=tls::Tls::prepare(&mapped,&bytes)?;\n    if backend.is_some() && !tls.data.is_empty(){return Err("packaged TLS setup is not supported yet".into());}\n    let mut unresolved = Vec::new();\n    let mut slots=Vec::new();');s=s.replace('            unresolved.push(format!("{}!{name}", import.dll));','            unresolved.push(format!("{}!{name}", import.dll));\n            slots.push(import.iat_rva);');a=s.index('    if !unresolved.is_empty() {');b=s.index('    mapped.protect()?;',a);s=s[:a]+'''    let traps=traps::Traps::new(unresolved)?;
    for (index,rva) in slots.iter().enumerate(){mapped.bind(*rva,traps.address(index))?;}
    if backend.is_none(){game_imports::initialize_windows_thread(mapped.base(),tls.data.clone())?;}
'''+s[b:];s=s.replace('    unsafe {\n        mapped.enter', '    tls.attach(mapped.base());\n    unsafe {\n        mapped.enter');p.write_text(s)
