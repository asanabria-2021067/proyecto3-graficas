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

# El sonido sale del mismo recorrido (docs/frames/audio.wav). Si se pasa una canción,
# se mezcla abajo del sonido de la nave.
AUDIO=docs/frames/audio.wav
if [ $# -ge 1 ]; then
  ffmpeg -y -framerate 30 -i docs/frames/%05d.png -i "$AUDIO" -i "$1" \
    -filter_complex "[2:a]volume=0.35,afade=t=out:st=92:d=5[m];[1:a][m]amix=inputs=2:duration=first,loudnorm=I=-16[a]" \
    -map 0:v -map "[a]" -c:v libx264 -preset veryfast -crf 27 -pix_fmt yuv420p -c:a aac -b:a 192k -shortest docs/recorrido.mp4
else
  ffmpeg -y -framerate 30 -i docs/frames/%05d.png -i "$AUDIO" \
    -af "loudnorm=I=-16" -c:v libx264 -preset veryfast -crf 27 -pix_fmt yuv420p -c:a aac -b:a 192k -shortest docs/recorrido.mp4
fi

# Versión liviana (720p, menos de 10 MB) para arrastrarla al editor del README en GitHub:
# así GitHub la guarda como adjunto y la muestra con reproductor dentro del README.
ffmpeg -y -i docs/recorrido.mp4 -vf scale=1280:-2 -c:v libx264 -preset veryfast -b:v 700k -pass 1 -an -f mp4 /dev/null
ffmpeg -y -i docs/recorrido.mp4 -vf scale=1280:-2 -c:v libx264 -preset veryfast -b:v 700k -pass 2 -c:a aac -b:a 64k docs/recorrido_readme.mp4
rm -f ffmpeg2pass-0.log ffmpeg2pass-0.log.mbtree

# GIF corto (la llegada a Tessara y la serpiente) para el README y Discord.
ffmpeg -y -ss 9 -t 12 -i docs/recorrido.mp4 \
  -vf "fps=12,scale=480:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=bayer:bayer_scale=3" \
  docs/recorrido.gif

echo "Listo: docs/recorrido.mp4 (1080p), docs/recorrido_readme.mp4 (720p) y docs/recorrido.gif"
