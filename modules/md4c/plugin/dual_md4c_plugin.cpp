// Adattatore: espone MD4C (libreria stock di Homebrew, nessun fork,
// nessuna ricompilazione: MIT non impone isolamento, questo modulo esiste
// solo per coerenza architetturale — stesso motivo di dual-adplug/
// dual-openmpt in homebrew-dual-audio) dietro dual_viewer_plugin.h.

#define DUAL_VIEWER_PLUGIN_BUILDING
#include "dual_viewer_plugin.h"

#include <md4c-html.h>

#include <cstdlib>
#include <cstring>
#include <string>

namespace {

struct md4c_plugin_state_t {
    char last_error[256] {};
};

static const char* const kExtensions[] = { "md", "markdown", nullptr };

void* DUAL_VIEWER_PLUGIN_ABI md4c_create(void) {
    return new md4c_plugin_state_t();
}

void DUAL_VIEWER_PLUGIN_ABI md4c_destroy(void* self) {
    delete static_cast<md4c_plugin_state_t*>(self);
}

const char* DUAL_VIEWER_PLUGIN_ABI md4c_last_error(void* self) {
    return static_cast<md4c_plugin_state_t*>(self)->last_error;
}

int DUAL_VIEWER_PLUGIN_ABI md4c_can_handle(void*, const char* path_hint) {
    if(!path_hint) return 0;
    for(const char* const* e = kExtensions; *e; e++)
        if(strcmp(path_hint, *e) == 0) return 1;
    return 0;
}

// md_html() e' basata su callback (chunk di output ripetuti), non su un
// buffer singolo come cmark — l'accumulo in una std::string e' compito di
// questo adattatore, l'ABI non lo sa e non le importa.
void OnChunk(const MD_CHAR* data, MD_SIZE size, void* userdata) {
    static_cast<std::string*>(userdata)->append(data, size);
}

int DUAL_VIEWER_PLUGIN_ABI md4c_render(void* self,
                                        const char* content, size_t content_len,
                                        const char* /*path_hint*/,
                                        char** out_html, size_t* out_html_len) {
    auto* s = static_cast<md4c_plugin_state_t*>(self);
    std::string accum;
    const int rc = md_html(content, static_cast<MD_SIZE>(content_len), OnChunk, &accum,
                            0, MD_HTML_FLAG_SKIP_UTF8_BOM);
    if(rc != 0) {
        snprintf(s->last_error, sizeof(s->last_error), "md_html() failed");
        return 0;
    }
    *out_html = static_cast<char*>(malloc(accum.size()));
    memcpy(*out_html, accum.data(), accum.size());
    *out_html_len = accum.size();
    return 1;
}

void DUAL_VIEWER_PLUGIN_ABI md4c_free_result(void*, char* html) {
    free(html);
}

const dual_viewer_plugin_t kPlugin = {
    /* info */ {
        DUAL_VIEWER_PLUGIN_TYPE_TEXT_TO_HTML,
        DUAL_VIEWER_PLUGIN_API_VMAJOR,
        DUAL_VIEWER_PLUGIN_API_VMINOR,
        0, 5, // MD4C 0.5.x (stock Homebrew)
        "md4c",
        "MD4C",
        "Markdown (CommonMark + estensioni GFM) verso HTML",
        "MD4C: Markdown parser for C\n"
        "Copyright (c) 2016-2024 Martin Mitas\n"
        "MIT License\n"
        "https://github.com/mity/md4c",
        "https://github.com/siriokds/homebrew-dual-viewers",
        kExtensions,
        nullptr, // config_dialog: nessuna opzione oggi
        nullptr, // extended_params
    },
    /* ops */ {
        md4c_create, md4c_destroy, md4c_last_error, md4c_can_handle,
        md4c_render, md4c_free_result,
    },
};

} // namespace

extern "C" DUAL_VIEWER_PLUGIN_EXPORT const dual_viewer_plugin_t* DUAL_VIEWER_PLUGIN_ABI
dual_viewer_plugin_load(void) {
    return &kPlugin;
}
