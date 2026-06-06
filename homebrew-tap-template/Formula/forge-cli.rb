# Homebrew formula for forge-cli
# This file is automatically updated by the release workflow

class TipCli < Formula
  desc "LiteForge CLI for AI development"
  homepage "https://github.com/seanpoyner/liteforge"
  version "0.1.0"
  license "MIT"

  # SHA256 checksums are updated automatically by the release workflow
  on_macos do
    on_arm do
      url "https://github.com/seanpoyner/liteforge/releases/download/v0.1.0/forge-cli-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
    on_intel do
      url "https://github.com/seanpoyner/liteforge/releases/download/v0.1.0/forge-cli-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/seanpoyner/liteforge/releases/download/v0.1.0/forge-cli-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
    on_intel do
      url "https://github.com/seanpoyner/liteforge/releases/download/v0.1.0/forge-cli-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  def install
    bin.install "forge"
  end

  def caveats
    <<~EOS
      To complete the setup, run:
        forge config init

      Then configure your API key:
        forge config set-secret forge-api-key

      For more information, visit:
        https://github.com/seanpoyner/liteforge
    EOS
  end

  test do
    assert_match "forge", shell_output("#{bin}/forge --version")
  end
end
