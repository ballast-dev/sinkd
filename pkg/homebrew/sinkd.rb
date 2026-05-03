# Homebrew formula (macOS): native Darwin `cargo install`, plus configs from cfg/system and cfg/user.
#
# Install from latest main (no release tarball checksum):
#   brew install --head ./pkg/homebrew/sinkd.rb
#
# For a versioned install after tagging vX.Y.Z on GitHub, add:
#   url "https://github.com/ballast-dev/sinkd/archive/refs/tags/vX.Y.Z.tar.gz"
#   sha256 "<sha256 from: curl -sL ... | shasum -a 256>"
# and remove or keep `head` as you prefer.

class Sinkd < Formula
  desc "Local sync daemon wrapping rsync"
  homepage "https://github.com/ballast-dev/sinkd"
  head "https://github.com/ballast-dev/sinkd.git", branch: "main"

  depends_on "rust" => :build
  depends_on "rsync"

  def install
    system "cargo", "install", *std_cargo_args(path: "client")
    system "cargo", "install", *std_cargo_args(path: "server")
    etc.install buildpath / "cfg/system/sinkd.conf" => "sinkd.conf"
    (share / "sinkd").install buildpath / "cfg/user/sinkd.conf" => "sinkd.user.conf"
  end

  test do
    assert_predicate bin / "sinkd", :exist?
    assert_predicate bin / "sinkd-srv", :exist?
    assert_path_exists etc / "sinkd.conf"
    assert_path_exists share / "sinkd/sinkd.user.conf"
  end
end
