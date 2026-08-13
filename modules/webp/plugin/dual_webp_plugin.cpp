// Adattatore: espone libwebp (libreria stock di Homebrew, nessun fork,
// nessuna ricompilazione: BSD-3-Clause non impone isolamento, questo modulo
// esiste solo per coerenza architetturale — stesso motivo di dual-md4c) dietro
// dual_viewer_plugin.h, type IMAGE_DECODE.

#define DUAL_VIEWER_PLUGIN_BUILDING
#include "dual_viewer_plugin.h"

#include <webp/decode.h>

#include <cstdio>
#include <cstring>

namespace {

struct webp_plugin_state_t {
    char last_error[256] {};
};

static const char* const kExtensions[] = { "webp", nullptr };

void* DUAL_VIEWER_PLUGIN_ABI webp_create(void) {
    return new webp_plugin_state_t();
}

void DUAL_VIEWER_PLUGIN_ABI webp_destroy(void* self) {
    delete static_cast<webp_plugin_state_t*>(self);
}

const char* DUAL_VIEWER_PLUGIN_ABI webp_last_error(void* self) {
    return static_cast<webp_plugin_state_t*>(self)->last_error;
}

int DUAL_VIEWER_PLUGIN_ABI webp_can_handle(void*, const char* path_hint) {
    return path_hint && strcmp(path_hint, "webp") == 0;
}

// WebPDecodeRGBA() produce gia' esattamente il layout richiesto dall'ABI:
// RGBA8 non premoltiplicato, row-major, top-down, stride = width*4, senza
// padding — l'adattatore e' un passthrough, nessuna conversione da fare.
int DUAL_VIEWER_PLUGIN_ABI webp_decode(void* self,
                                        const uint8_t* data, size_t data_len,
                                        const char* /*path_hint*/,
                                        uint8_t** out_rgba,
                                        int* out_width, int* out_height) {
    auto* s = static_cast<webp_plugin_state_t*>(self);
    uint8_t* rgba = WebPDecodeRGBA(data, data_len, out_width, out_height);
    if(!rgba) {
        snprintf(s->last_error, sizeof(s->last_error), "WebPDecodeRGBA() failed");
        return 0;
    }
    *out_rgba = rgba;
    return 1;
}

void DUAL_VIEWER_PLUGIN_ABI webp_free_result(void*, uint8_t* rgba) {
    WebPFree(rgba);
}

const dual_viewer_plugin_t kPlugin = {
    /* info */ {
        DUAL_VIEWER_PLUGIN_TYPE_IMAGE_DECODE,
        DUAL_VIEWER_PLUGIN_API_VMAJOR,
        DUAL_VIEWER_PLUGIN_API_VMINOR,
        1, 6, // libwebp 1.6.x (stock Homebrew)
        "webp",
        "WebP",
        "WebP image decode (lossy + lossless) to RGBA",
        "libwebp: WebP codec\n"
        "Copyright (c) 2010, Google Inc. All rights reserved.\n"
        "BSD-3-Clause License\n"
        "https://chromium.googlesource.com/webm/libwebp",
        "https://github.com/siriokds/homebrew-dual-viewers",
        kExtensions,
        nullptr, // config_dialog: nessuna opzione oggi
        nullptr, // extended_params
    },
    /* ops */ { .image_decode = {
        webp_create, webp_destroy, webp_last_error, webp_can_handle,
        webp_decode, webp_free_result,
    }},
};

} // namespace

extern "C" DUAL_VIEWER_PLUGIN_EXPORT const dual_viewer_plugin_t* DUAL_VIEWER_PLUGIN_ABI
dual_viewer_plugin_load(void) {
    return &kPlugin;
}
