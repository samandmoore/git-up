class GitUp < Formula
  version '0.1.0'
  desc 'Git up command to fetch and update all local branches that track remotes.'
  homepage 'https://github.com/samandmoore/git-up'

  if OS.mac?
    url "https://github.com/samandmoore/git-up/releases/download/#{version}/git-up-#{version}-x86_64-apple-darwin.tar.gz"
    sha256 'something'
  end

  def install
    bin.install 'git-up'
  end
end
