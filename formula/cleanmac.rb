class Cleanmac < Formula
  desc "A fast, AI-ready macOS system cleaner CLI with TUI"
  homepage "https://github.com/wis-graph/cleanmac-cli"
  version "1.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/wis-graph/cleanmac-cli/releases/download/v#{version}/cleanmac-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
    on_intel do
      url "https://github.com/wis-graph/cleanmac-cli/releases/download/v#{version}/cleanmac-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  def install
    bin.install "cleanmac"
  end

  test do
    assert_match "cleanmac", shell_output("#{bin}/cleanmac --version")
  end
end
