class MediaManagerCli < Formula
  desc "Gestor de contenido multimedia para terminal (indexador local + reproductor mpv/vlc)"
  homepage "https://github.com/Angel-p-v/media-manager"
  version "X.Y.Z"
  license "MIT"

  if OS.mac? && Hardware::CPU.arm?
    url "https://github.com/Angel-p-v/media-manager/releases/download/v#{version}/media-manager-cli-#{version}-darwin-arm64.tar.gz"
  elsif OS.mac?
    url "https://github.com/Angel-p-v/media-manager/releases/download/v#{version}/media-manager-cli-#{version}-darwin-x64.tar.gz"
  end

  # Nota: los assets macOS están pendientes de runners de GitHub Actions.
  # Mientras tanto, se puede instalar desde el bundle portable de Linux
  # o desde el script install.sh.

  def install
    bin.install "mm"
  end

  test do
    system "#{bin}/mm", "--version"
  end
end
