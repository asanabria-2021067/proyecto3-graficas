#!/usr/bin/env bash
# Arma el video y el GIF del README a partir de los cuadros de `cargo run --release -- tour`.
# Uso:  bash tools/make_video.sh [musica.mp3]
# Necesita ffmpeg.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ ! -d docs/frames ]; then
  echo "Primero corre: cargo run --release -- tour" >&2
  exit 1
fi

if [ $# -ge 1 ]; then
  # Con música: se corta al largo del video y se baja el volumen al final.
  ffmpeg -y -framerate 30 -i docs/frames/%05d.png -i "$1" \
    -c:v libx264 -preset veryfast -crf 27 -pix_fmt yuv420p \
    -c:a aac -b:a 160k -af "afade=t=out:st=72:d=4" -shortest docs/recorrido.mp4
else
  ffmpeg -y -framerate 30 -i docs/frames/%05d.png \
    -c:v libx264 -preset veryfast -crf 27 -pix_fmt yuv420p docs/recorrido.mp4
fi

# GIF corto (la llegada a Tessara y la serpiente) para el README y Discord.
ffmpeg -y -ss 9 -t 12 -i docs/recorrido.mp4 \
  -vf "fps=12,scale=480:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=bayer:bayer_scale=3" \
  docs/recorrido.gif

echo "Listo: docs/recorrido.mp4 y docs/recorrido.gif"
