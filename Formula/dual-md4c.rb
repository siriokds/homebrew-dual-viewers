class DualMd4c < Formula
  desc "Plugin adapter for the stock MD4C library, for Dual"
  homepage "https://github.com/siriokds/homebrew-dual-viewers/tree/main/modules/md4c"
  # Nessun sorgente esterno da scaricare: l'unico "sorgente" e' l'adattatore
  # stesso, che vive in questo stesso tap (modules/md4c/plugin/, letto via
  # __dir__ in install — stesso schema di dual-adplug in homebrew-dual-audio).
  # Il "url" qui e' solo formale — Homebrew lo richiede sempre — non e' cio'
  # che install() usa davvero.
  #
  # MD4C e' MIT: nessun obbligo di isolamento, il link diretto e' gia'
  # legale. L'unico motivo di questo modulo e' togliere l'header di MD4C
  # dalla compilazione di Dual — Dual parla solo con dual_viewer_plugin.h.
  url "https://github.com/siriokds/homebrew-dual-viewers.git", branch: "main"
  version "1.0.0"
  license "MIT"

  # Stessa convenzione degli altri moduli dual-*: vive sotto
  # /opt/homebrew/opt/dual-<nome>/lib/, che e' cio' che il loader lato Dual
  # scandisce.
  keg_only "kept under its own prefix, same convention as the other dual-* modules"

  livecheck do
    skip "il sorgente e' questo stesso repository, nessuna versione upstream da tracciare"
  end

  depends_on "md4c"

  def install
    plugin_src = Pathname.new(__dir__).parent/"modules/md4c/plugin"
    system ENV.cxx, "-std=c++17", "-shared", "-fPIC",
           "-fvisibility=hidden", "-fvisibility-inlines-hidden",
           "-I#{Formula["md4c"].opt_include}",
           "-L#{Formula["md4c"].opt_lib}", "-lmd4c", "-lmd4c-html",
           "-Wl,-rpath,#{Formula["md4c"].opt_lib}",
           "-o", "libdual_md4c_plugin.dylib",
           plugin_src/"dual_md4c_plugin.cpp"
    lib.mkdir
    lib.install "libdual_md4c_plugin.dylib"
  end

  test do
    assert_predicate lib/"libdual_md4c_plugin.dylib", :exist?
  end
end
