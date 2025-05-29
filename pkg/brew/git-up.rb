class GitUp < Formula
  version 'v0.2.0'
  desc 'Git up command to fetch and update all local branches that track remotes.'
  homepage 'https://github.com/samandmoore/git-up'

  if OS.mac?
    url "https://github.com/samandmoore/git-up/releases/download/#{version}/git-up-#{version}-aarch64-apple-darwin.tar.gz"
    sha256 'ce8cfd066f2cb348a8500479f1bf3b6ae4e33bcfc73cb026c663cadfed54b394'
  end

  def install
    bin.install 'git-up'
  end
end
