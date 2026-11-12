//! Sonido sintetizado: no hay archivos de audio, todo sale de osciladores y ruido filtrado.
//! Lo usa la ventana (en vivo) y también el recorrido del video (se escribe a un .wav),
//! así el video suena igual que el juego.
//!
//! Capas:
//! - Ambiente del espacio: un zumbido grave con varias notas desafinadas que respiran
//!   despacio, un "viento" de ruido muy grave y un brillo agudo que aparece y desaparece.
//! - Motor: ruido grave cuyo filtro se abre con el acelerador, más un zumbido que sube de
//!   tono con la velocidad. Con turbo se agrega un silbido.
//! - Giro: ruido pasado por un filtro de banda que suena del lado hacia donde se gira.
//! - Choque: un golpe grave que se apaga rápido.
use std::f32::consts::TAU;

pub const SAMPLE_RATE: u32 = 48_000;

/// Lo que el sintetizador necesita saber del juego. Valores ya normalizados.
#[derive(Clone, Copy, Debug, Default)]
pub struct SoundParams {
    pub throttle: f32, // 0..1 qué tanto empuja el motor
    pub speed: f32,    // 0..1 velocidad respecto a la máxima con turbo
    pub turn: f32,     // -1..1 giro (+ = derecha)
    pub boost: bool,
    pub impact: f32, // 0..1 fuerza del último choque (sube de golpe y baja sola)
}

/// Filtro pasa bajos de un polo.
#[derive(Default, Clone, Copy)]
struct LowPass {
    y: f32,
}

impl LowPass {
    fn step(&mut self, x: f32, cutoff: f32) -> f32 {
        let a = 1.0 - (-TAU * cutoff / SAMPLE_RATE as f32).exp();
        self.y += a * (x - self.y);
        self.y
    }
}

/// Filtro de estado variable (sirve como pasa banda con resonancia).
#[derive(Default, Clone, Copy)]
struct BandPass {
    low: f32,
    band: f32,
}

impl BandPass {
    fn step(&mut self, x: f32, freq: f32, q: f32) -> f32 {
        let f = 2.0 * (std::f32::consts::PI * freq / SAMPLE_RATE as f32).sin();
        let high = x - self.low - self.band / q;
        self.band += f * high;
        self.low += f * self.band;
        self.band
    }
}

pub struct Synth {
    t: f64,
    seed: u32,
    p: SoundParams, // parámetros suavizados (sin saltos que truenen)
    target: SoundParams,
    last_impact: f32,
    thump: f32,
    boost_level: f32, // turbo suavizado (0..1)
    phases: [f32; 8],
    wind: LowPass,
    rumble: LowPass,
    rumble2: LowPass,
    whoosh_l: BandPass,
    whoosh_r: BandPass,
    hiss: BandPass,
    pub volume: f32,
}

impl Default for Synth {
    fn default() -> Self {
        Self::new()
    }
}

impl Synth {
    pub fn new() -> Self {
        Self {
            t: 0.0,
            seed: 0x1234_5678,
            p: SoundParams::default(),
            target: SoundParams::default(),
            last_impact: 0.0,
            thump: 0.0,
            boost_level: 0.0,
            phases: [0.0; 8],
            wind: LowPass::default(),
            rumble: LowPass::default(),
            rumble2: LowPass::default(),
            whoosh_l: BandPass::default(),
            whoosh_r: BandPass::default(),
            hiss: BandPass::default(),
            volume: 0.8,
        }
    }

    pub fn set(&mut self, p: SoundParams) {
        // Un choque nuevo se nota porque el impacto sube de golpe.
        if p.impact > self.last_impact + 0.05 {
            self.thump = self.thump.max(p.impact);
        }
        self.last_impact = p.impact;
        self.target = p;
    }

    /// Ruido blanco en [-1, 1] (xorshift).
    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn osc(&mut self, i: usize, freq: f32) -> f32 {
        self.phases[i] = (self.phases[i] + freq / SAMPLE_RATE as f32).fract();
        (self.phases[i] * TAU).sin()
    }

    /// Llena `out` con muestras estéreo intercaladas (izquierda, derecha, ...).
    pub fn render(&mut self, out: &mut [f32]) {
        let dt = 1.0 / SAMPLE_RATE as f32;
        let k = 1.0 - (-dt * 8.0).exp(); // suavizado de los parámetros (~120 ms)
        for frame in out.chunks_exact_mut(2) {
            self.t += dt as f64;
            let t = self.t as f32;
            let tg = self.target;
            self.p.throttle += (tg.throttle - self.p.throttle) * k;
            self.p.speed += (tg.speed - self.p.speed) * k;
            self.p.turn += (tg.turn - self.p.turn) * k;
            let boost_target = if tg.boost { 1.0 } else { 0.0 };
            self.boost_level += (boost_target - self.boost_level) * k * 0.5;
            let boost = self.boost_level;
            let p = self.p;

            // ---------- Ambiente ----------
            let breathe = 0.6 + 0.4 * (t * 0.13).sin();
            let drone = (self.osc(0, 55.0) + self.osc(1, 55.35) * 0.8 + self.osc(2, 82.6) * 0.5) * 0.06 * breathe;
            let n = self.noise();
            let wind = self.wind.step(n, 120.0 + 60.0 * (t * 0.07).sin()) * 0.35 * (0.5 + 0.5 * (t * 0.05).sin());
            let shimmer_amp = ((t * 0.09).sin() * 0.5 + 0.5).powi(3) * 0.012;
            let shimmer = (self.osc(3, 660.0) + self.osc(4, 990.5) * 0.6) * shimmer_amp;
            let ambient = drone + wind + shimmer;

            // ---------- Motor ----------
            let n = self.noise();
            let cutoff = 150.0 + p.throttle * 900.0 + p.speed * 600.0;
            let rumble = self.rumble2.step(self.rumble.step(n, cutoff), cutoff) * (p.throttle * 1.6 + p.speed * 0.4);
            let pitch = 48.0 + p.speed * 90.0 + p.throttle * 20.0;
            let hum = (self.osc(5, pitch) + self.osc(6, pitch * 2.01) * 0.4) * 0.05 * (p.throttle * 0.8 + p.speed * 0.5);
            let n = self.noise();
            let whine = (self.osc(7, 380.0 + p.speed * 500.0) * 0.03 + self.hiss.step(n, 2500.0, 2.0) * 0.05) * boost;
            let engine = rumble + hum + whine;

            // ---------- Giro: ruido en el lado del giro ----------
            let amount = p.turn.abs();
            let n = self.noise();
            let freq = 400.0 + amount * 1400.0;
            let wl = self.whoosh_l.step(n, freq, 3.0);
            let wr = self.whoosh_r.step(n, freq * 1.05, 3.0);
            let pan = p.turn.clamp(-1.0, 1.0) * 0.5 + 0.5; // 0 = izquierda, 1 = derecha
            let whoosh = amount * 0.12;

            // ---------- Choque ----------
            let thump = (self.thump * (TAU * 50.0 * t).sin() + self.thump * self.noise() * 0.3) * 0.5;
            self.thump *= (-dt * 6.0).exp();

            let mono = ambient + engine + thump;
            let l = mono + wl * whoosh * (1.0 - pan) * 1.4;
            let r = mono + wr * whoosh * pan * 1.4;
            // Saturación suave: nunca pasa de 1 aunque se sumen muchas capas.
            frame[0] = (l * self.volume).tanh();
            frame[1] = (r * self.volume).tanh();
        }
    }
}

/// Guarda muestras estéreo (en [-1, 1]) como .wav de 16 bits.
pub fn write_wav(path: &str, samples: &[f32]) -> std::io::Result<()> {
    use std::io::Write;
    let data_len = (samples.len() * 2) as u32;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?; // PCM
    f.write_all(&2u16.to_le_bytes())?; // estéreo
    f.write_all(&SAMPLE_RATE.to_le_bytes())?;
    f.write_all(&(SAMPLE_RATE * 4).to_le_bytes())?;
    f.write_all(&4u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        f.write_all(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())?;
    }
    f.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_sonido_nunca_se_sale_de_rango() {
        let mut s = Synth::new();
        s.set(SoundParams { throttle: 1.0, speed: 1.0, turn: 1.0, boost: true, impact: 1.0 });
        let mut buf = vec![0.0; 48_000];
        s.render(&mut buf);
        assert!(buf.iter().all(|x| x.abs() <= 1.0 && x.is_finite()));
    }

    #[test]
    fn el_motor_suena_mas_fuerte_al_acelerar() {
        let energy = |throttle: f32| {
            let mut s = Synth::new();
            s.set(SoundParams { throttle, speed: throttle * 0.5, ..Default::default() });
            let mut buf = vec![0.0; 48_000];
            s.render(&mut buf);
            buf[24_000..].iter().map(|x| x * x).sum::<f32>()
        };
        assert!(energy(1.0) > energy(0.0) * 2.0);
    }

    #[test]
    fn girar_a_la_derecha_suena_mas_a_la_derecha() {
        let mut s = Synth::new();
        s.set(SoundParams { turn: 1.0, ..Default::default() });
        let mut buf = vec![0.0; 96_000];
        s.render(&mut buf);
        let (mut l, mut r) = (0.0, 0.0);
        for f in buf[48_000..].chunks_exact(2) {
            l += f[0] * f[0];
            r += f[1] * f[1];
        }
        assert!(r > l * 1.2);
    }
}
