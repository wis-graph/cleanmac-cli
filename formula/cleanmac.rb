class Cleanmac < Formula
  desc "A fast, AI-ready macOS system cleaner CLI with TUI"
  homepage "https://github.com/wis-graph/cleanmac-cli"
  version "1.0.3"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/wis-graph/cleanmac-cli/releases/download/v#{version}/cleanmac-aarch64-apple-darwin.tar.gz"
      sha256 "fd97dc363307878bddd8b8cc979390556c64685dcdf8dc09c80df9dc3d856a18"
    end
    on_intel do
      url "https://github.com/wis-graph/cleanmac-cli/releases/download/v#{version}/cleanmac-x86_64-apple-darwin.tar.gz"
      sha256 "d617266f0260f4c3f0740db2ab481094ff1fea1fcbc435670d6d87345e5a4d7b"
    end
  end

  def install
    bin.install "cleanmac"
  end

  test do
    assert_match "cleanmac", shell_output("#{bin}/cleanmac --version")
  end
end
