class Cleanmac < Formula
  desc "A fast, AI-ready macOS system cleaner CLI with TUI"
  homepage "https://github.com/wis-graph/cleanmac-cli"
  version "1.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/wis-graph/cleanmac-cli/releases/download/v#{version}/cleanmac-aarch64-apple-darwin.tar.gz"
      sha256 "b1ca98728e7be4068662f7f07cf8f6dd39ddaa1756edca9cb2a47dccef375f0a"
    end
    on_intel do
      url "https://github.com/wis-graph/cleanmac-cli/releases/download/v#{version}/cleanmac-x86_64-apple-darwin.tar.gz"
      sha256 "d82b048f453962b7ac361b527bd7a377b20b2b2661a4e4e8ad0fb05ecab7a47e"
    end
  end

  def install
    bin.install "cleanmac"
  end

  test do
    assert_match "cleanmac", shell_output("#{bin}/cleanmac --version")
  end
end
