class DualSyntax < Formula
  desc "Code colours for Dual's text viewer (tree-sitter), as a viewer plugin"
  homepage "https://github.com/siriokds/homebrew-dual-viewers/tree/main/modules/syntax"
  # Come dual-md4c: il sorgente e' il modulo in questo stesso tap
  # (modules/syntax/, letto via __dir__ in install); l'url e' solo formale.
  # Il modulo e' in Rust (tree-sitter e le grammatiche, dentro la dylib):
  # Rust serve solo per compilare, e con la bottle nemmeno quello.
  url "https://github.com/siriokds/homebrew-dual-viewers.git", branch: "main"
  version "1.0.0"
  license "GPL-3.0-or-later"

  # Stessa convenzione degli altri moduli dual-*: Dual cerca in
  # /opt/homebrew/opt/dual-<nome>/lib/.
  keg_only "kept under its own prefix, same convention as the other dual-* modules"

  livecheck do
    skip "il sorgente e' questo stesso repository, nessuna versione upstream da tracciare"
  end

  depends_on "rust" => :build

  def install
    # Una copia del modulo, cosi' la compilazione non scrive nel tap.
    module_src = Pathname.new(__dir__).parent/"modules/syntax"
    (buildpath/"syntax").mkpath
    cp_r module_src.children.reject { |p| p.basename.to_s == "target" }, buildpath/"syntax"
    system "cargo", "build", "--release", "--locked",
           "--manifest-path", buildpath/"syntax/Cargo.toml",
           "--target-dir", buildpath/"target"
    lib.install buildpath/"target/release/libdual_syntax_plugin.dylib"
  end

  test do
    assert_predicate lib/"libdual_syntax_plugin.dylib", :exist?
  end
end
