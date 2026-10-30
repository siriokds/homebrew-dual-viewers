//! Il plugin dei visualizzatori (`dual_viewer_plugin.h`, ABI 1.2, tipo
//! SYNTAX_HIGHLIGHT): le stesse strutture campo per campo, misurate sul C
//! nei test. Un solo simbolo esportato, `dual_viewer_plugin_load`.

use std::ffi::{CStr, CString, c_char, c_int, c_void};

use crate::highlight::{highlight, language_for};

const API_VMAJOR: u16 = 1;
const API_VMINOR: u16 = 2;
const TYPE_SYNTAX_HIGHLIGHT: c_int = 2;

/// `dual_viewer_plugin_info_t`.
#[repr(C)]
struct Info {
    kind: c_int,
    api_vmajor: u16,
    api_vminor: u16,
    version_major: u16,
    version_minor: u16,
    id: *const c_char,
    name: *const c_char,
    descr: *const c_char,
    copyright: *const c_char,
    website: *const c_char,
    extensions: *const *const c_char,
    config_dialog: *const c_char,
    extended_params: *const c_void,
}

/// `dual_syntax_span_t`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Span {
    start: usize,
    len: usize,
    role: i32,
}

/// `dual_viewer_plugin_syntax_ops_t`.
#[repr(C)]
struct SyntaxOps {
    create: unsafe extern "C" fn() -> *mut c_void,
    destroy: unsafe extern "C" fn(*mut c_void),
    last_error: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    can_handle: unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int,
    highlight: unsafe extern "C" fn(
        *mut c_void,
        *const c_char,
        usize,
        *const c_char,
        *mut *const Span,
        *mut usize,
    ) -> c_int,
    free_result: unsafe extern "C" fn(*mut c_void, *const Span),
}

/// `dual_viewer_plugin_t` con il solo membro della union che questo tipo usa
/// (le tre forme hanno la stessa misura, 6 puntatori).
#[repr(C)]
struct Plugin {
    info: Info,
    ops: SyntaxOps,
}

// Tutto statico e mai modificato: si può condividere fra i thread.
unsafe impl Sync for Plugin {}
unsafe impl Send for Plugin {}

/// L'elenco C delle estensioni, terminato da NULL.
struct Extensions([*const c_char; crate::EXTENSIONS.len() + 1]);
unsafe impl Sync for Extensions {}
unsafe impl Send for Extensions {}

static EXTENSIONS_C: std::sync::OnceLock<(Vec<CString>, Extensions)> = std::sync::OnceLock::new();

fn extensions() -> *const *const c_char {
    let (_, list) = EXTENSIONS_C.get_or_init(|| {
        let owned: Vec<CString> = crate::EXTENSIONS.iter().map(|e| CString::new(*e).unwrap()).collect();
        let mut list = [std::ptr::null(); crate::EXTENSIONS.len() + 1];
        for (slot, e) in list.iter_mut().zip(&owned) {
            *slot = e.as_ptr();
        }
        (owned, Extensions(list))
    });
    list.0.as_ptr()
}

/// Un'istanza: l'ultimo errore e gli ultimi tratti, che restano del plugin
/// fino a `free_result` o alla prossima `highlight`.
#[derive(Default)]
struct Instance {
    error: CString,
    spans: Vec<Span>,
}

unsafe extern "C" fn create() -> *mut c_void {
    Box::into_raw(Box::<Instance>::default()).cast()
}

unsafe extern "C" fn destroy(this: *mut c_void) {
    if !this.is_null() {
        drop(unsafe { Box::from_raw(this.cast::<Instance>()) });
    }
}

unsafe extern "C" fn last_error(this: *mut c_void) -> *const c_char {
    match unsafe { this.cast::<Instance>().as_ref() } {
        Some(instance) => instance.error.as_ptr(),
        None => c"no instance".as_ptr(),
    }
}

/// L'estensione senza punto (NULL o non UTF-8: nessuna).
unsafe fn hint(path_hint: *const c_char) -> String {
    if path_hint.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(path_hint) }.to_string_lossy().into_owned()
}

unsafe extern "C" fn can_handle(_this: *mut c_void, path_hint: *const c_char) -> c_int {
    c_int::from(language_for(&unsafe { hint(path_hint) }).is_some())
}

unsafe extern "C" fn highlight_c(
    this: *mut c_void,
    content: *const c_char,
    content_len: usize,
    path_hint: *const c_char,
    out_spans: *mut *const Span,
    out_count: *mut usize,
) -> c_int {
    let Some(instance) = (unsafe { this.cast::<Instance>().as_mut() }) else {
        return 0;
    };
    if out_spans.is_null() || out_count.is_null() || (content.is_null() && content_len > 0) {
        instance.error = c"null argument".into();
        return 0;
    }
    let ext = unsafe { hint(path_hint) };
    let Some(language) = language_for(&ext) else {
        instance.error = CString::new(format!("no language for \"{ext}\"")).unwrap_or_default();
        return 0;
    };
    let text: &[u8] = if content_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(content.cast::<u8>(), content_len) }
    };
    // Un panic dentro tree-sitter non deve attraversare il confine C.
    let Ok(spans) = std::panic::catch_unwind(|| highlight(language, text)) else {
        instance.error = c"the analysis stopped".into();
        return 0;
    };
    instance.spans = spans
        .into_iter()
        .map(|s| Span {
            start: s.start,
            len: s.len,
            role: s.role as i32,
        })
        .collect();
    unsafe {
        *out_spans = instance.spans.as_ptr();
        *out_count = instance.spans.len();
    }
    1
}

unsafe extern "C" fn free_result(this: *mut c_void, _spans: *const Span) {
    if let Some(instance) = unsafe { this.cast::<Instance>().as_mut() } {
        instance.spans = Vec::new();
    }
}

static PLUGIN: std::sync::OnceLock<Plugin> = std::sync::OnceLock::new();

/// L'unico simbolo esportato.
#[unsafe(no_mangle)]
pub extern "C" fn dual_viewer_plugin_load() -> *const c_void {
    let plugin = PLUGIN.get_or_init(|| Plugin {
        info: Info {
            kind: TYPE_SYNTAX_HIGHLIGHT,
            api_vmajor: API_VMAJOR,
            api_vminor: API_VMINOR,
            version_major: 1,
            version_minor: 0,
            id: c"syntax".as_ptr(),
            name: c"Tree-sitter".as_ptr(),
            descr: c"Code colours in the text viewer (tree-sitter)".as_ptr(),
            copyright: c"tree-sitter, tree-sitter-highlight and the grammars: MIT License, \
                         their authors. This adapter: GPL-3.0-or-later."
                .as_ptr(),
            website: c"https://tree-sitter.github.io".as_ptr(),
            extensions: extensions(),
            config_dialog: std::ptr::null(),
            extended_params: std::ptr::null(),
        },
        ops: SyntaxOps {
            create,
            destroy,
            last_error,
            can_handle,
            highlight: highlight_c,
            free_result,
        },
    });
    (plugin as *const Plugin).cast()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    /// I numeri del C, compilato da `dual_viewer_plugin.h` 1.2 (31/10/2026,
    /// `sizeof`/`offsetof` su arm64).
    #[test]
    fn same_layout_as_the_c_header() {
        assert_eq!(size_of::<Info>(), 80);
        assert_eq!(size_of::<SyntaxOps>(), 48);
        assert_eq!(size_of::<Plugin>(), 128);
        assert_eq!(size_of::<Span>(), 24);
        assert_eq!(offset_of!(SyntaxOps, highlight), 32);
        assert_eq!(offset_of!(SyntaxOps, free_result), 40);
        assert_eq!(offset_of!(Span, role), 16);
    }

    /// Dall'inizio alla fine attraverso le funzioni C, come le chiama Dual.
    #[test]
    fn through_the_c_functions() {
        let plugin = unsafe { &*dual_viewer_plugin_load().cast::<Plugin>() };
        assert_eq!((plugin.info.api_vmajor, plugin.info.api_vminor), (1, 2));
        let ops = &plugin.ops;
        unsafe {
            let this = (ops.create)();
            assert_eq!((ops.can_handle)(this, c"rs".as_ptr()), 1);
            assert_eq!((ops.can_handle)(this, c"txt".as_ptr()), 0);
            let text = "fn f() {} // x\n";
            let (mut spans, mut count) = (std::ptr::null(), 0usize);
            let ok = (ops.highlight)(this, text.as_ptr().cast(), text.len(), c"rs".as_ptr(), &mut spans, &mut count);
            assert_eq!(ok, 1);
            let spans = std::slice::from_raw_parts(spans, count);
            assert_eq!(spans.iter().map(|s| s.len).sum::<usize>(), text.len());
            assert_eq!(spans[0].role, crate::highlight::Role::Keyword as i32);
            (ops.free_result)(this, spans.as_ptr());
            // Un'estensione che non si colora: l'errore, nessun panic.
            let ok = (ops.highlight)(this, text.as_ptr().cast(), text.len(), c"txt".as_ptr(), &mut std::ptr::null(), &mut count);
            assert_eq!(ok, 0);
            assert!(CStr::from_ptr((ops.last_error)(this)).to_string_lossy().contains("txt"));
            (ops.destroy)(this);
        }
    }

    #[test]
    fn extensions_end_with_null() {
        let list = extensions();
        let mut n = 0;
        unsafe {
            while !(*list.add(n)).is_null() {
                n += 1;
            }
        }
        assert_eq!(n, crate::EXTENSIONS.len());
    }
}
