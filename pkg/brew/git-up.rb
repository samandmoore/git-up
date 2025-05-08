class GitUp < Formula
  version 'v0.2.0'
  desc 'Git up command to fetch and update all local branches that track remotes.'
  homepage 'https://github.com/samandmoore/git-up'

  if OS.mac?
    url "https://github.com/samandmoore/git-up/releases/download/#{version}/git-up-#{version}-aarch64-apple-darwin.tar.gz"
    sha256 'e4d230d180857dfa3ccea26176f666cf3e9d2652ee841d5a9b7c20e3768b6d39'
  end

  def install
    bin.install 'git-up'
  end
end
