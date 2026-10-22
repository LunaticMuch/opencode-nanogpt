class OpencodeNanogpt < Formula
  desc "Configure OpenCode to use NanoGPT models and MCP tools"
  homepage "https://github.com/LunaticMuch/opencode-nanogpt"
  version "1.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/LunaticMuch/opencode-nanogpt/releases/download/v#{version}/opencode-nanogpt-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "FILL_AFTER_RELEASE"
    end
    on_intel do
      url "https://github.com/LunaticMuch/opencode-nanogpt/releases/download/v#{version}/opencode-nanogpt-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "FILL_AFTER_RELEASE"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/LunaticMuch/opencode-nanogpt/releases/download/v#{version}/opencode-nanogpt-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "FILL_AFTER_RELEASE"
    end
    on_intel do
      url "https://github.com/LunaticMuch/opencode-nanogpt/releases/download/v#{version}/opencode-nanogpt-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "FILL_AFTER_RELEASE"
    end
  end

  def install
    bin.install "opencode-nanogpt"
  end

  test do
    assert_match "opencode-nanogpt", shell_output("#{bin}/opencode-nanogpt --version")
    assert_match "init", shell_output("#{bin}/opencode-nanogpt --help")
  end
end
