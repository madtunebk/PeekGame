use crate::model::{Dependency, Module};
use std::{fmt::Write as _, path::Path};
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                write!(out, "\\u{:04x}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn path_json(p: &Path) -> String {
    quote(&p.to_string_lossy())
}

pub fn render(modules: &[Module], deps: &[Dependency]) -> String {
    let mut out = String::from(
        r#"{
  "schema_version": 2,
  "analysis_kind": "static_pe_imports",
  "runtime_complete": false,
  "implementation_status": "rust_registry_checked",
  "limitations": ["Runtime LoadLibrary/GetProcAddress requests require tracing", "API arguments and COM vtable calls are not inferred", "External DLLs may be supplied by a compatibility backend; external does not mean missing"],
  "modules": ["#,
    );
    for (n, m) in modules.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        write!(out,"\n    {{\"path\": {}, \"machine\": {}, \"architecture\": \"x86_64\", \"file_bytes\": {}, \"image_base\": {}, \"entry_rva\": {}, \"imports\": [",
            path_json(&m.path),m.image.machine,m.bytes,m.image.image_base,m.image.entry_rva).unwrap();
        for (n, i) in m.image.imports.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let export = game_imports::registry::find(&i.dll, i.name.as_deref().unwrap_or(""));
            write!(out,"\n      {{\"dll\": {}, \"name\": {}, \"ordinal\": {}, \"hint\": {}, \"iat_rva\": {}, \"kind\": {}, \"backend_status\": {}, \"category\": {}, \"scope\": {}}}",
                quote(&i.dll),i.name.as_deref().map(quote).unwrap_or("null".into()),
                i.ordinal.map(|n|n.to_string()).unwrap_or("null".into()),
                i.hint.map(|n|n.to_string()).unwrap_or("null".into()),i.iat_rva,
                quote(if i.delayed{"delay"}else{"normal"}),
                quote(export.map(|e|e.status).unwrap_or("unknown")),
                export.map(|e|quote(e.category)).unwrap_or("null".into()),
                export.map(|e|quote(e.scope)).unwrap_or("null".into())).unwrap();
        }
        out.push_str("\n    ]}");
    }
    out.push_str("\n  ],\n  \"dependencies\": [");
    for (n, d) in deps.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        write!(
            out,
            "\n    {{\"importer\": {}, \"dll\": {}, \"status\": {}, \"path\": {}}}",
            path_json(&d.importer),
            quote(&d.dll),
            quote(if d.resolved.is_some() {
                "local_pe"
            } else {
                "external"
            }),
            d.resolved
                .as_deref()
                .map(path_json)
                .unwrap_or("null".into())
        )
        .unwrap();
    }
    out.push_str("\n  ]\n}\n");
    out
}
