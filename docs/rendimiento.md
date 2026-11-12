# Rendimiento: descarte y niveles de detalle

Generado con `cargo run --release -- capture rendimiento` (640x360, promedio de 5 cuadros).
"Triángulos enviados" son los que llegan al pipeline; los objetos descartados no mandan ninguno.

## Sobre Tessara (superficie)

| Descarte | LOD | Objetos dibujados | Objetos descartados | Triángulos enviados | Triángulos rasterizados | ms por cuadro |
|---|---|---|---|---|---|---|
| sí | sí | 150 | 88 | 31044 | 6767 | 79.2 |
| no | sí | 239 | 0 | 39196 | 6767 | 84.6 |
| sí | no | 415 | 88 | 113480 | 46602 | 95.5 |
| no | no | 647 | 0 | 207388 | 46602 | 93.7 |

## Cerca de Kharon

| Descarte | LOD | Objetos dibujados | Objetos descartados | Triángulos enviados | Triángulos rasterizados | ms por cuadro |
|---|---|---|---|---|---|---|
| sí | sí | 59 | 61 | 6858 | 1437 | 25.9 |
| no | sí | 121 | 0 | 11548 | 1437 | 26.6 |
| sí | no | 59 | 61 | 22218 | 3319 | 28.8 |
| no | no | 647 | 0 | 207388 | 3319 | 32.0 |

## Inicio, en el espacio

| Descarte | LOD | Objetos dibujados | Objetos descartados | Triángulos enviados | Triángulos rasterizados | ms por cuadro |
|---|---|---|---|---|---|---|
| sí | sí | 10 | 3 | 4304 | 1711 | 27.1 |
| no | sí | 14 | 0 | 6544 | 1711 | 25.2 |
| sí | no | 436 | 3 | 123538 | 53484 | 36.2 |
| no | no | 609 | 0 | 195228 | 53484 | 40.9 |

