# Proyecto 3: Space Travel

Sistema solar inventado que se explora con una nave, dibujado con un **software renderer propio en Rust**. raylib solo se usa para abrir la ventana, leer el teclado y el mouse y mostrar el framebuffer ya terminado. Todo lo demás (transformaciones, recorte, rasterizado, depth test, shaders, culling y niveles de detalle) está hecho a mano en este repo.

![recorrido](docs/recorrido.gif)

🎬 **Video completo:** [docs/recorrido.mp4](docs/recorrido.mp4): vuelo por el espacio, llegada a Tessara, vuelta junto a la serpiente de roca, paso por la luna Nix, visita a Mirra y a Orvane, salida y vista del sistema.

| Vista del sistema | Superficie de Tessara |
|---|---|
| ![](docs/sistema.png) | ![](docs/tessara_superficie.png) |

**Integrante:** Angel Sanabria (carné 24725). Trabajo individual.

---

## Cómo correrlo

Requisitos: Rust (edición 2024) y lo que necesita raylib para compilar (CMake y un compilador de C). En Linux además: `libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev`.

```bash
cargo run --release
```

> Usar siempre `--release`: en debug el ruido de los shaders es muchísimo más lento.

Otros modos (no abren ventana):

```bash
cargo run --release -- capture              # capturas del README en docs/
cargo run --release -- capture rendimiento  # tabla de docs/rendimiento.md
cargo run --release -- tour                 # cuadros del video en docs/frames/
bash tools/make_video.sh [musica.mp3]       # arma docs/recorrido.mp4 y el GIF (necesita ffmpeg)
```

## Controles

| Tecla | Acción |
|---|---|
| **W / S** | Acelerar / reversa (el empuje sube y baja de forma progresiva) |
| **A / D** | Girar a la izquierda / derecha |
| **Flechas ↑ / ↓** | Subir / bajar la nariz |
| **Q / E** | Bajar / subir |
| **Espacio** | Frenar |
| **Shift** | Turbo |
| **Tab** | Entrar o salir del **modo local** (rodear el planeta más cercano) |
| **1 - 7** | Piloto automático: Ignis, Tessara, Kharon, Glacia, Orvane, Mirra y Nix (la luna) |
| **V** | Vista del sistema desde arriba |
| **Mouse derecho** | Mirar alrededor de la nave · **Rueda**: distancia de la cámara |
| **C / L / N** | Encender o apagar el descarte, los niveles de detalle y el mapeo de normales |
| **G / P** | Gravedad / pausar las órbitas |
| **O / M / H** | Órbitas / minimapa / ayuda |
| **F1** | Resolución completa, media o un tercio (para computadoras lentas) |
| **F2 / K** | Ocultar los textos (para grabar) / guardar captura |

**Parámetros en vivo** (se aplican al planeta más cercano o al del modo local; sirven para la demostración):

| Tecla | Parámetro |
|---|---|
| **9 / 0** | Radio del planeta |
| **, / .** | Intensidad del shader (nubes de Tessara, calor de Ignis, tormenta de Orvane, aurora de Glacia, relieve de Kharon, flores de Mirra, manchas del sol) |
| **- / =** | Velocidad orbital (multiplicador del tiempo de las órbitas) |
| **[ / ]** | Semilla de la distribución de objetos (cambia dónde salen los árboles, casas, etc.) |

En la parte de abajo de la ventana se ven los valores actuales y los contadores de triángulos y objetos.

---

## El sistema

Todos los cuerpos están en el plano eclíptico (y = 0). Las órbitas son circulares, cerca del sol para que el viaje sea corto, y las posiciones dependen del tiempo transcurrido, no de los cuadros. Nix es una luna: da vueltas alrededor de Tessara mientras Tessara da vueltas al sol.

| Cuerpo | Qué tiene |
|---|---|
| **Solmira** (sol) | Plasma animado con ruido 4D, granulación, manchas solares, oscurecimiento del borde y halo |
| **Ignis** | Planeta de lava: placas de corteza (Voronoi) y lava que fluye por las grietas y en los mares bajos. Rocas encima |
| **Tessara** | Planeta rocoso: océanos, biomas, hielo, nubes con sombra, luces de ciudad de noche. Árboles, casas, rocas y **la serpiente de roca** |
| **Kharon** | Planeta sin atmósfera con la **geometría deformada** (montañas y cráteres). Obeliscos y rocas |
| **Glacia** | Hielo agrietado con mapa de normales, mares congelados que brillan y aurora en los polos. Cristales |
| **Orvane** | Gigante gaseoso con bandas que fluyen y una tormenta, más un anillo con huecos y la sombra del planeta |
| **Mirra** | Planeta miniatura estilo caricatura: luz en tres tonos, contorno, camino en espiral. Hongos gigantes, árboles y un faro |
| **Nix** | Luna de Tessara, también deformada pero con otra semilla de terreno (otros cráteres y montañas) |

| Ignis | Tessara | Kharon |
|---|---|---|
| ![](docs/ignis.png) | ![](docs/tessara.png) | ![](docs/kharon.png) |
| **Glacia** | **Orvane** | **Mirra** |
| ![](docs/glacia.png) | ![](docs/orvane.png) | ![](docs/mirra.png) |

| La serpiente de roca de Tessara | Nix, la luna |
|---|---|
| ![](docs/serpiente.png) | ![](docs/nix.png) |
