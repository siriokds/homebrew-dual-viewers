// Interfaccia C stabile per i "viewer plugin" di Dual — content testuale
// dentro, frammento HTML fuori (quello che poi finisce in una wxWebView).
// Stessa filosofia di dual_audio_plugin.h nel repo homebrew-dual-audio:
// un solo simbolo esportato per plugin, versione dell'API separata dalla
// versione del plugin, nessuna dipendenza su C++ ABI o su un GUI toolkit.
//
// Validata con una prova di fattibilita' fuori da Dual prima di scrivere
// questo header (tre librerie reali con API interne del tutto diverse —
// callback a chunk, buffer singolo, stream C++ — dietro le stesse 6
// funzioni: create/destroy/last_error/can_handle/render/free_result).

#pragma once
#include <stdint.h>
#include <stddef.h>

#if defined(_WIN32)
    #define DUAL_VIEWER_PLUGIN_ABI __cdecl
    #if defined(DUAL_VIEWER_PLUGIN_BUILDING)
        #define DUAL_VIEWER_PLUGIN_EXPORT __declspec(dllexport)
    #else
        #define DUAL_VIEWER_PLUGIN_EXPORT __declspec(dllimport)
    #endif
#else
    #define DUAL_VIEWER_PLUGIN_ABI
    #define DUAL_VIEWER_PLUGIN_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define DUAL_VIEWER_PLUGIN_API_VMAJOR 1
#define DUAL_VIEWER_PLUGIN_API_VMINOR 1

// TEXT_TO_HTML (contenuto testuale dentro, frammento HTML fuori) e
// IMAGE_DECODE (bytes di un file immagine dentro, pixel RGBA fuori — per i
// formati che wxWidgets non decodifica da solo, es. WebP). Ogni type ha la
// propria vtable in dual_viewer_plugin_ops_u: stesso ruolo di
// dual_audio_plugin_type_t nell'ABI audio, dove GENERATOR/EFFECT/OUTPUT
// selezionano forme diverse di ops. Il loader legge type PRIMA di
// interpretare ops.
typedef enum {
    DUAL_VIEWER_PLUGIN_TYPE_TEXT_TO_HTML = 0,
    DUAL_VIEWER_PLUGIN_TYPE_IMAGE_DECODE = 1,
} dual_viewer_plugin_type_t;

typedef struct {
    dual_viewer_plugin_type_t type;

    uint16_t api_vmajor;
    uint16_t api_vminor;

    uint16_t version_major;   // versione del plugin/libreria sottostante
    uint16_t version_minor;

    const char* id;           // "md4c" — usato internamente, stabile
    const char* name;         // "MD4C" — mostrato all'utente
    const char* descr;        // una riga
    const char* copyright;    // licenza per esteso, multi-riga
    const char* website;      // opzionale, puo' essere NULL

    // Estensioni gestite (SENZA il punto, minuscole), terminata da NULL —
    // stessa convenzione di dual_audio_plugin_t.
    const char* const* extensions;

    // Mini-DSL alla DeaDBeeF, stesso formato di dual_audio_plugin.h v1.3
    // ("select|etichetta|chiave_config|chiave_default|k1:d1|k2:d2|...").
    // NULL se il plugin non ha parametri configurabili.
    const char* config_dialog;

    // Estensione futura (pNext-style) — vedi lo stesso campo in
    // dual_audio_plugin.h per il ragionamento completo. DEVE essere NULL
    // finche' non esiste una v2 di questa struct.
    const void* extended_params;
} dual_viewer_plugin_info_t;

typedef struct {
    void* (DUAL_VIEWER_PLUGIN_ABI *create)(void);
    void  (DUAL_VIEWER_PLUGIN_ABI *destroy)(void* self);

    // Stringa statica/thread-local del plugin, valida solo fino alla
    // chiamata successiva — l'host la copia subito se deve mostrarla.
    const char* (DUAL_VIEWER_PLUGIN_ABI *last_error)(void* self);

    // path_hint e' SOLO l'estensione senza punto, minuscola ("md", "c",
    // "py", ...) — mai un path reale: il plugin non tocca mai il
    // filesystem, l'host gli passa gia' il contenuto letto.
    int (DUAL_VIEWER_PLUGIN_ABI *can_handle)(void* self, const char* path_hint);

    // content/content_len: il file GIA' letto dall'host (il plugin non
    // apre mai un file — stesso principio per cui un plugin audio non apre
    // mai un device). out_html e' allocata dal plugin (qualunque sia il suo
    // allocatore interno, purche' free_result() sappia liberarla — mai
    // assumere che sia malloc() lato host, un plugin Windows potrebbe usare
    // un heap diverso). Ritorna 0/1, non un codice d'errore: il messaggio
    // sta in last_error().
    int (DUAL_VIEWER_PLUGIN_ABI *render)(void* self,
                                          const char* content, size_t content_len,
                                          const char* path_hint,
                                          char** out_html, size_t* out_html_len);
    void (DUAL_VIEWER_PLUGIN_ABI *free_result)(void* self, char* html);
} dual_viewer_plugin_ops_t;

// Vtable per DUAL_VIEWER_PLUGIN_TYPE_IMAGE_DECODE. Stessa forma di
// dual_viewer_plugin_ops_t (create/destroy/last_error/can_handle + una
// funzione di lavoro + free_result), decode() al posto di render(): stesso
// principio, un formato diverso in uscita.
typedef struct {
    void* (DUAL_VIEWER_PLUGIN_ABI *create)(void);
    void  (DUAL_VIEWER_PLUGIN_ABI *destroy)(void* self);
    const char* (DUAL_VIEWER_PLUGIN_ABI *last_error)(void* self);
    int (DUAL_VIEWER_PLUGIN_ABI *can_handle)(void* self, const char* path_hint);

    // data/data_len: il file immagine GIA' letto dall'host, cosi' come
    // content/content_len in render() — il plugin non tocca mai il
    // filesystem. out_rgba e' allocata dal plugin: RGBA8 non premoltiplicato,
    // row-major, top-down, stride = width*4, senza padding — un pixel buffer
    // pronto per wxImage, nessun formato immagine, nessun header. Ritorna
    // 0/1, il messaggio sta in last_error().
    int (DUAL_VIEWER_PLUGIN_ABI *decode)(void* self,
                                          const uint8_t* data, size_t data_len,
                                          const char* path_hint,
                                          uint8_t** out_rgba,
                                          int* out_width, int* out_height);
    void (DUAL_VIEWER_PLUGIN_ABI *free_result)(void* self, uint8_t* rgba);
} dual_viewer_plugin_image_ops_t;

// Quale membro e' valido lo decide info.type — mai leggere il membro
// sbagliato, i due non hanno lo stesso significato anche se qui la forma
// (6 puntatori) coincide.
typedef union {
    dual_viewer_plugin_ops_t       text_to_html;
    dual_viewer_plugin_image_ops_t image_decode;
} dual_viewer_plugin_ops_u;

typedef struct dual_viewer_plugin_s {
    dual_viewer_plugin_info_t info;
    dual_viewer_plugin_ops_u  ops;
} dual_viewer_plugin_t;

// Unico simbolo esportato dal modulo (.dylib/.so/.dll), risolto via
// dlsym/GetProcAddress. Il plugin lo dichiara cosi':
//   DUAL_VIEWER_PLUGIN_EXPORT const dual_viewer_plugin_t* DUAL_VIEWER_PLUGIN_ABI
//   dual_viewer_plugin_load(void) { ... }
typedef const dual_viewer_plugin_t* (DUAL_VIEWER_PLUGIN_ABI *dual_viewer_plugin_load_fn)(void);
#define DUAL_VIEWER_PLUGIN_ENTRY_SYMBOL "dual_viewer_plugin_load"

#ifdef __cplusplus
}
#endif
