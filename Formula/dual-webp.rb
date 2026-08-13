class DualWebp < Formula
  desc "Plugin adapter for the stock libwebp library, for Dual"
  homepage "https://github.com/siriokds/homebrew-dual-viewers/tree/main/modules/webp"
  # Nessun sorgente esterno da scaricare: l'unico "sorgente" e' l'adattatore
  # stesso, che vive in questo stesso tap (modules/webp/plugin/, letto via
  # __dir__ in install — stesso schema di dual-md4c).
  # Il "url" qui e' solo formale — Homebrew lo richiede sempre — non e' cio'
  # che install() usa davvero.
  #
  # libwebp e' BSD-3-Clause: nessun obbligo di isolamento, il link diretto e'
  # gia' legale. L'unico motivo di questo modulo e' togliere l'header di
  # libwebp dalla compilazione di Dual — Dual parla solo con
  # dual_viewer_plugin.h.
  url "https://github.com/siriokds/homebrew-dual-viewers.git", branch: "main"
  version "1.0.0"
  license "BSD-3-Clause"

  # Stessa convenzione degli altri moduli dual-*: vive sotto
  # /opt/homebrew/opt/dual-<nome>/lib/, che e' cio' che il loader lato Dual
  # scandisce.
  keg_only "kept under its own prefix, same convention as the other dual-* modules"

  livecheck do
    skip "il sorgente e' questo stesso repository, nessuna versione upstream da tracciare"
  end

  depends_on "webp"

  def install
    plugin_src = Pathname.new(__dir__).parent/"modules/webp/plugin"
    system ENV.cxx, "-std=c++20", "-shared", "-fPIC",
           "-fvisibility=hidden", "-fvisibility-inlines-hidden",
           "-I#{Formula["webp"].opt_include}",
           "-L#{Formula["webp"].opt_lib}", "-lwebp",
           "-Wl,-rpath,#{Formula["webp"].opt_lib}",
           "-o", "libdual_webp_plugin.dylib",
           plugin_src/"dual_webp_plugin.cpp"
    lib.mkdir
    lib.install "libdual_webp_plugin.dylib"
  end

  test do
    assert_predicate lib/"libdual_webp_plugin.dylib", :exist?
  end
end
