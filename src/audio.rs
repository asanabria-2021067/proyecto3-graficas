//! Salida de audio en vivo. raylib solo pone las muestras en la tarjeta de sonido: el sonido
//! lo calcula `synth.rs`. Cada vez que raylib terminó de tocar un bloque se le manda el
//! siguiente.
use crate::synth::{SAMPLE_RATE, SoundParams, Synth};
use raylib::ffi;

const BLOCK: usize = 1024; // cuadros (izquierda + derecha) por bloque

pub struct Audio {
    stream: ffi::AudioStream,
    synth: Synth,
    buffer: Vec<f32>,
    pub muted: bool,
}

impl Audio {
    /// Devuelve `None` si no hay tarjeta de sonido (el juego sigue sin audio).
    pub fn new() -> Option<Self> {
        unsafe {
            ffi::InitAudioDevice();
            if !ffi::IsAudioDeviceReady() {
                return None;
            }
            ffi::SetAudioStreamBufferSizeDefault(BLOCK as i32);
            let stream = ffi::LoadAudioStream(SAMPLE_RATE, 32, 2);
            ffi::PlayAudioStream(stream);
            Some(Self { stream, synth: Synth::new(), buffer: vec![0.0; BLOCK * 2], muted: false })
        }
    }

    /// Se llama una vez por cuadro con el estado actual de la nave.
    pub fn update(&mut self, params: SoundParams) {
        self.synth.set(params);
        self.synth.volume = if self.muted { 0.0 } else { 0.8 };
        unsafe {
            while ffi::IsAudioStreamProcessed(self.stream) {
                self.synth.render(&mut self.buffer);
                ffi::UpdateAudioStream(self.stream, self.buffer.as_ptr() as *const std::ffi::c_void, BLOCK as i32);
            }
        }
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        unsafe {
            ffi::UnloadAudioStream(self.stream);
            ffi::CloseAudioDevice();
        }
    }
}
