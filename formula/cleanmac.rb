class Cleanmac < Formula
  desc "A fast, AI-ready macOS system cleaner CLI with TUI"
  homepage "https://github.com/wis-graph/cleanmac-cli"
  version "1.0.2"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/wis-graph/cleanmac-cli/releases/download/v#{version}/cleanmac-aarch64-apple-darwin.tar.gz"
      sha256 "058fc534cd931fc0721bcb770f1bc43e890da2164126b83a67858eeab6d3f161"
    end
    on_intel do
      url "https://github.com/wis-graph/cleanmac-cli/releases/download/v#{version}/cleanmac-x86_64-apple-darwin.tar.gz"
      sha256 "bf4b5ba6b12347c6eb42f5e28b67226c543af13624063f68cbd46a6c00ce9080"
    end
  end

  def install
    bin.install "cleanmac"
  end

  test do
    assert_match "cleanmac", shell_output("#{bin}/cleanmac --version")
  end
end
