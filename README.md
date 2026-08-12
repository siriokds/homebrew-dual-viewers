# homebrew-dual-viewers

Homebrew tap for the *viewer* modules used by
[Dual](https://github.com/siriokds/dual) as external plugins — stesso
principio del tap gemello
[homebrew-dual-audio](https://github.com/siriokds/homebrew-dual-audio), ma
per i visualizzatori invece dei backend audio. Ogni modulo espone una
libreria di rendering (contenuto testuale dentro, frammento HTML fuori)
dietro `dual_viewer_plugin.h`, un'interfaccia C stabile che Dual carica a
runtime con `dlopen` — mai un `#include` diretto della libreria vera dentro
Dual.

Validata con una prova di fattibilità fuori da Dual prima di scrivere
l'header: tre librerie reali con API interne del tutto diverse (callback a
chunk, buffer singolo, stream C++ con eccezioni) dietro le stesse 6 funzioni
(`create`/`destroy`/`last_error`/`can_handle`/`render`/`free_result`).

## Modules

- **`modules/md4c/`** — Markdown (CommonMark + estensioni GFM) verso HTML,
  via [MD4C](https://github.com/mity/md4c) (MIT). Nessun obbligo di
  isolamento — il modulo esiste solo per togliere l'header di MD4C dalla
  compilazione di Dual, non per motivi di licenza. `depends_on` la formula
  stock di Homebrew, solo l'adattatore è nostro. Formula:
  `Formula/dual-md4c.rb`.

Altri candidati esplorati ma non ancora moduli reali (vedi la
conversazione che ha portato a questo tap): `cmark` (implementazione di
riferimento CommonMark, stessa forma di MD4C) e GNU `source-highlight`
(evidenziazione sintattica di codice sorgente, GPL-3.0-or-later — motivo
per cui questo repository è licenziato GPL-3.0-or-later e non v2 come il
tap audio, nel caso entri in futuro).

## Installing

```sh
brew tap siriokds/dual-viewers
brew install siriokds/dual-viewers/dual-md4c
```

## Dove finiscono le cose, e perché `keg_only`

Ogni formula qui è `keg_only`, per la stessa convenzione del tap audio:
`/opt/homebrew/opt/dual-<nome>/lib/` è esattamente ciò che il loader lato
Dual scandisce.

```sh
/opt/homebrew/opt/dual-md4c/lib/libdual_md4c_plugin.dylib
```

## License

**GNU General Public License v3.0-or-later** — vedi [LICENSE](LICENSE).
Nessun modulo oggi obbliga a questa scelta (MD4C è MIT), ma è la stessa
licenza "or-later" che coprirebbe senza conflitti un futuro modulo
GPL-3.0-or-later come source-highlight, mentre GPL v2 puro no. Le
`Formula/*.rb` non richiederebbero singolarmente GPL (nessun codice del
modulo, solo istruzioni di build — stessa ragione per cui `homebrew-core`
resta BSD-2-Clause pur ospitando formule per software GPL), ma sono coperte
dalla stessa licenza del repository per semplicità.
