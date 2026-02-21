class OpLoader < Formula
  desc "TUI and CLI tool for managing 1Password secrets as environment variables"
  homepage "https://github.com/idiomattic/op-loader"
  version "0.4.3"

  on_macos do
    on_arm do
      url "https://github.com/idiomattic/op-loader/releases/download/v#{version}/op-loader-macos-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "op-loader"
  end

  test do
    system "#{bin}/op-loader", "--help"
  end
end
