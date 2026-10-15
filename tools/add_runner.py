from pathlib import Path
files={
'src/loader/mod.rs':r'''mod mapping;
use crate::pe;
use std::{ffi::CString, fs, path::Path};
pub fn run(path: &Path, args: &[String], output: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let image = pe::parse(&bytes)?;
    if image.imports.iter().any(|i| i.delayed) { return Err("delay imports are not supported by this runner yet".into()); }
    let mapped = mapping::Mapping::new(&bytes)?;
    let trace = output.join("dynamic-imports.tsv");
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    game_imports::start_import_trace(&trace).map_err(|e| e.to_string())?;
    let line = std::iter::once(path.to_string_lossy().into_owned()).chain(args.iter().cloned())
        .map(|a| quote(&a)).collect::<Vec<_>>().join(" ");
    let line = CString::new(line).map_err(|e| e.to_string())?;
    let code = unsafe { game_imports::peekgame_configure_process(mapped.base(), line.as_ptr()) };
    if code != 0 { return Err(format!("process configuration failed: {code}")); }
    let mut unresolved = Vec::new();
    for import in &image.imports {
        let Some(name) = &import.name else { return Err("ordinal static imports are not supported yet".into()); };
        let dll = CString::new(import.dll.as_str()).map_err(|e| e.to_string())?;
        let symbol = CString::new(name.as_str()).map_err(|e| e.to_string())?;
        let address = unsafe { game_imports::peekgame_resolve(dll.as_ptr(), symbol.as_ptr()) };
        if address.is_null() { unresolved.push(format!("{}!{name}", import.dll)); }
        else { mapped.bind(import.iat_rva, address as usize)?; }
    }
    if !unresolved.is_empty() { return Err(format!("unresolved static imports: {}", unresolved.join(", "))); }
    mapped.protect()?;
    eprintln!("PeekGame: entering {} at {:#x}; dynamic requests -> {}", path.display(), mapped.base() + image.entry_rva as usize, trace.display());
    unsafe { mapped.enter(image.entry_rva)?; }
    Ok(())
}
fn quote(s: &str) -> String {
    let mut out = String::from("\""); let mut slashes = 0;
    for ch in s.chars() {
        if ch == '\\' { slashes += 1; continue; }
        if ch == '"' { out.push_str(&"\\".repeat(slashes * 2 + 1)); }
        else { out.push_str(&"\\".repeat(slashes)); }
        slashes = 0; out.push(ch);
    }
    out.push_str(&"\\".repeat(slashes * 2)); out.push('"'); out
}
''',
'src/loader/mapping.rs':r'''use std::ffi::c_void;
unsafe extern "C" {
    fn mmap(address: *mut c_void, size: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
    fn mprotect(address: *mut c_void, size: usize, prot: i32) -> i32;
    fn munmap(address: *mut c_void, size: usize) -> i32;
}
struct Section { start: usize, size: usize, protection: i32 }
pub struct Mapping { address: *mut u8, size: usize, headers: usize, sections: Vec<Section> }
fn u32_at(b: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(b.get(at..at+4).ok_or("truncated PE header")?.try_into().unwrap()))
}
impl Mapping {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        let pe = u32_at(bytes, 60)? as usize; let opt = pe + 24;
        let count = u16::from_le_bytes(bytes[pe+6..pe+8].try_into().unwrap()) as usize;
        let opt_size = u16::from_le_bytes(bytes[pe+20..pe+22].try_into().unwrap()) as usize;
        let preferred = u64::from_le_bytes(bytes[opt+24..opt+32].try_into().unwrap()) as usize;
        let size = u32_at(bytes,opt+56)? as usize; let headers = u32_at(bytes,opt+60)? as usize;
        if size == 0 || size > 1024*1024*1024 || preferred % 4096 != 0 || headers > size { return Err("invalid image extent/base".into()); }
        let directories = u32_at(bytes,opt+108)? as usize;
        if directories > 9 && u32_at(bytes,opt+112+9*8)? != 0 { return Err("TLS is not supported by this prototype runner yet".into()); }
        // MAP_FIXED_NOREPLACE never overwrites existing Linux mappings.
        let raw = unsafe { mmap(preferred as *mut c_void,size,3,0x100022,-1,0) };
        if raw as isize == -1 { return Err(format!("cannot reserve preferred PE base: {}",std::io::Error::last_os_error())); }
        if raw as usize != preferred { unsafe {munmap(raw,size); } return Err("kernel did not honor preferred PE base".into()); }
        let mut result = Self {address:raw.cast(),size,headers,sections:Vec::new()};
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(),result.address,headers); }
        for index in 0..count {
            let at = opt + opt_size + index*40;
            let start = u32_at(bytes,at+12)? as usize;
            let raw_size = u32_at(bytes,at+16)? as usize;
            let file = u32_at(bytes,at+20)? as usize;
            let extent = (u32_at(bytes,at+8)? as usize).max(raw_size);
            if start % 4096 != 0 || start.checked_add(extent).is_none_or(|end| end > size) { return Err("invalid section extent/alignment".into()); }
            let data = bytes.get(file..file.checked_add(raw_size).ok_or("section overflow")?).ok_or("truncated section")?;
            unsafe {std::ptr::copy_nonoverlapping(data.as_ptr(),result.address.add(start),raw_size);}
            let flags = u32_at(bytes,at+36)?;
            let protection = ((flags & 0x40000000 != 0) as i32) | (((flags & 0x80000000 != 0) as i32)<<1) | (((flags & 0x20000000 != 0) as i32)<<2);
            if protection & 6 == 6 { return Err("writable executable PE sections are unsupported".into()); }
            result.sections.push(Section{start,size:extent,protection});
        }
        Ok(result)
    }
    pub fn base(&self) -> usize {self.address as usize}
    pub fn bind(&self,rva:u32,value:usize)->Result<(),String> {
        let at=rva as usize;
        if at.checked_add(8).is_none_or(|end|end>self.size) {return Err("IAT slot outside image".into());}
        unsafe {self.address.add(at).cast::<usize>().write_unaligned(value);} Ok(())
    }
    pub fn protect(&self)->Result<(),String> {
        let apply=|start,size,prot| {
            if size>0 && unsafe {mprotect(self.address.wrapping_add(start).cast(),size,prot)} != 0 {Err(std::io::Error::last_os_error().to_string())} else {Ok(())}
        };
        apply(0,self.size,0)?;apply(0,self.headers,1)?;
        for section in &self.sections {apply(section.start,section.size,section.protection)?;}
        Ok(())
    }
    pub unsafe fn enter(&self,rva:u32)->Result<(),String> {
        if !self.sections.iter().any(|s|s.protection&4!=0 && rva as usize>=s.start && (rva as usize)<s.start+s.size) {return Err("entry is not in an executable section".into());}
        let entry:unsafe extern "win64" fn()=unsafe {std::mem::transmute(self.address.add(rva as usize))};
        unsafe {entry();} Ok(())
    }
}
impl Drop for Mapping {fn drop(&mut self){unsafe {munmap(self.address.cast(),self.size);}}}
''',
'src/api/system/dynamic.rs':r'''use crate::{api::system::error, registry};
use std::{collections::BTreeMap, ffi::{c_char,c_void,CStr}, fs::File, io::Write, path::Path, sync::{Mutex,OnceLock}};
static MODULES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static TRACE: OnceLock<Mutex<Trace>> = OnceLock::new();
struct Trace {file:File, counts:BTreeMap<(String,String,String),usize>}
pub fn start(path:&Path)->std::io::Result<()> {
    let mut file=File::create(path)?;
    writeln!(file,"dll\texport\tstatus\trequests")?;
    TRACE.set(Mutex::new(Trace{file,counts:BTreeMap::new()})).map_err(|_|std::io::Error::other("trace already initialized"))
}
fn record(dll:&str,name:&str,status:&str) {
    if let Some(trace)=TRACE.get() {
        let mut t=trace.lock().unwrap();
        let key=(dll.to_owned(),name.to_owned(),status.to_owned());
        let count=t.counts.entry(key).or_default();*count+=1;let count=*count;
        if let Err(e)=writeln!(t.file,"{dll}\t{name}\t{status}\t{count}").and_then(|_|t.file.flush()) {eprintln!("dynamic import trace write failed: {e}");}
    }
}
fn modules()-> &'static Mutex<Vec<String>> {MODULES.get_or_init(||Mutex::new(Vec::new()))}
fn load(name:&str)->usize {
    let mut name=name.to_ascii_lowercase();if !name.ends_with(".dll"){name.push_str(".dll");}
    if !registry::entries().any(|e|e.dll==name) {record(&name,"<LoadLibrary>","module_not_found");error::set(126);return 0;}
    let mut list=modules().lock().unwrap();
    let index=if let Some(index)=list.iter().position(|s|s==&name){index}else{list.push(name.clone());list.len()-1};
    record(&name,"<LoadLibrary>","runtime_module");0x50470000+index*16
}
pub unsafe extern "win64" fn load_library_a(name:*const c_char)->usize {
    if name.is_null(){error::set(87);return 0;}
    match unsafe{CStr::from_ptr(name)}.to_str(){Ok(s)=>load(s),Err(_)=>{error::set(87);0}}
}
pub unsafe extern "win64" fn load_library_w(name:*const u16)->usize {
    if name.is_null(){error::set(87);return 0;}
    let mut text=Vec::new();for i in 0..32768 {let ch=unsafe{*name.add(i)};if ch==0{return match String::from_utf16(&text){Ok(s)=>load(&s),Err(_)=>{error::set(87);0}};}text.push(ch);}
    error::set(87);0
}
pub unsafe extern "win64" fn get_proc_address(module:usize,name:*const c_char)->*const c_void {
    let dll={let list=modules().lock().unwrap();module.checked_sub(0x50470000).filter(|v|v%16==0).and_then(|v|list.get(v/16)).cloned()};
    let Some(dll)=dll else{error::set(6);return std::ptr::null();};
    if name as usize<=65535 {record(&dll,&format!("#{}",name as usize),"ordinal_unsupported");error::set(127);return std::ptr::null();}
    let Ok(name)=unsafe{CStr::from_ptr(name)}.to_str() else{error::set(87);return std::ptr::null();};
    let address=registry::find(&dll,name).and_then(|e|e.address).map(|get|get()).unwrap_or(std::ptr::null());
    record(&dll,name,if address.is_null(){"not_found"}else{"implemented"});
    if address.is_null(){error::set(127);}address
}
'''
}
for name,text in files.items():
 p=Path(name);p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
p=Path('src/main.rs');p.write_text(p.read_text().replace('mod model;','mod model;\nmod loader;'))
p=Path('src/lib.rs');p.write_text(p.read_text()+'\npub use api::system::dynamic::start as start_import_trace;\n')
p=Path('src/api/system/mod.rs');p.write_text(p.read_text()+'\npub(crate) mod dynamic;\n')
p=Path('src/registry/system.rs');s=p.read_text().replace('    implemented!(','    implemented!(',1);s=s.replace('use super::*;','use super::*;\nuse crate::api::system::dynamic;');s=s.replace('pub(super) static EXPORTS: &[Export] = &[','pub(super) static EXPORTS: &[Export] = &[\n    implemented!("LoadLibraryA","system","partial","registered runtime modules only; external PE DLL loading unsupported",dynamic::load_library_a),\n    implemented!("LoadLibraryW","system","partial","registered runtime modules only; external PE DLL loading unsupported",dynamic::load_library_w),\n    implemented!("GetProcAddress","system","partial","named exports from registered runtime modules; logs requests",dynamic::get_proc_address),');p.write_text(s)
p=Path('src/cli.rs');s=p.read_text().replace('    pub exe: PathBuf,','    pub exe: PathBuf,\n    pub run: bool,\n    pub args: Vec<String>,');s=s.replace('    let (mut exe,','    let mut run = false;\n    let mut program_args = Vec::new();\n    let (mut exe,');s=s.replace('        if args[i] == "--analyze" {','        if args[i] == "--run" {\n            run = true;\n            program_args = args[i+1..].iter().map(|s|s.to_string_lossy().into_owned()).collect();\n            break;\n        } else if args[i] == "--analyze" {');s=s.replace('    if !flag {','    if !flag && !run {');s=s.replace('        exe,\n        output,','        exe,\n        run,\n        args: program_args,\n        output,');p.write_text(s)
p=Path('src/app.rs');s=p.read_text().replace('    let (modules, deps)', '    if options.run { return crate::loader::run(&options.exe, &options.args, &options.output); }\n    let (modules, deps)');p.write_text(s)
