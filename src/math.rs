//! Matemática básica: matrices 4x4 (fila por columna, vectores columna) y rotaciones.
use raylib::prelude::*;

pub type Mat4 = [[f32; 4]; 4];

pub fn identity() -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j] + a[i][3] * b[3][j];
        }
    }
    r
}

/// Multiplica la matriz por (v, w) y devuelve el vector homogéneo.
#[inline]
pub fn transform(m: &Mat4, v: Vector3, w: f32) -> [f32; 4] {
    [
        m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z + m[0][3] * w,
        m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z + m[1][3] * w,
        m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z + m[2][3] * w,
        m[3][0] * v.x + m[3][1] * v.y + m[3][2] * v.z + m[3][3] * w,
    ]
}

/// Transforma un punto (w = 1) y se queda con x, y, z.
#[inline]
pub fn transform_point(m: &Mat4, v: Vector3) -> Vector3 {
    let r = transform(m, v, 1.0);
    Vector3::new(r[0], r[1], r[2])
}

/// Transforma una dirección (w = 0): no le afecta la traslación.
#[inline]
pub fn transform_dir(m: &Mat4, v: Vector3) -> Vector3 {
    let r = transform(m, v, 0.0);
    Vector3::new(r[0], r[1], r[2])
}

pub fn translation(t: Vector3) -> Mat4 {
    let mut m = identity();
    m[0][3] = t.x;
    m[1][3] = t.y;
    m[2][3] = t.z;
    m
}

pub fn scale(s: f32) -> Mat4 {
    let mut m = identity();
    m[0][0] = s;
    m[1][1] = s;
    m[2][2] = s;
    m
}

pub fn rotation_y(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    [
        [c, 0.0, s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-s, 0.0, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn rotation_z(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    [
        [c, -s, 0.0, 0.0],
        [s, c, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Matriz de modelo a partir de una base (derecha, arriba, adelante) ya ortonormal.
/// El modelo se dibuja con +X a la derecha, +Y arriba y +Z hacia adelante.
pub fn from_basis(pos: Vector3, right: Vector3, up: Vector3, forward: Vector3, s: f32) -> Mat4 {
    [
        [right.x * s, up.x * s, forward.x * s, pos.x],
        [right.y * s, up.y * s, forward.y * s, pos.y],
        [right.z * s, up.z * s, forward.z * s, pos.z],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Modelo de un planeta = Traslación * Inclinación(Z) * Giro(Y) * Escala
pub fn model_matrix(t: Vector3, s: f32, tilt: f32, spin: f32) -> Mat4 {
    mul(&translation(t), &mul(&rotation_z(tilt), &mul(&rotation_y(spin), &scale(s))))
}

pub fn look_at(eye: Vector3, target: Vector3, up: Vector3) -> Mat4 {
    let f = (target - eye).normalized();
    let r = f.cross(up).normalized();
    let u = r.cross(f);
    [
        [r.x, r.y, r.z, -r.dot(eye)],
        [u.x, u.y, u.z, -u.dot(eye)],
        [-f.x, -f.y, -f.z, f.dot(eye)],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fovy / 2.0).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, (far + near) / (near - far), 2.0 * far * near / (near - far)],
        [0.0, 0.0, -1.0, 0.0],
    ]
}

pub fn rotate_y(v: Vector3, a: f32) -> Vector3 {
    let (s, c) = a.sin_cos();
    Vector3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z)
}

pub fn rotate_z(v: Vector3, a: f32) -> Vector3 {
    let (s, c) = a.sin_cos();
    Vector3::new(c * v.x - s * v.y, s * v.x + c * v.y, v.z)
}

/// Rotación de Rodrigues: gira `v` alrededor de `axis` (unitario) un ángulo `a`.
pub fn rotate_axis(v: Vector3, axis: Vector3, a: f32) -> Vector3 {
    let (s, c) = a.sin_cos();
    v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0 - c))
}

/// Suavizado exponencial que no depende de los FPS: `rate` = qué tan rápido llega.
#[inline]
pub fn damp(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    target + (current - target) * (-rate * dt).exp()
}

#[inline]
pub fn damp_v(current: Vector3, target: Vector3, rate: f32, dt: f32) -> Vector3 {
    target + (current - target) * (-rate * dt).exp()
}

/// Componente de `v` perpendicular a `n` (n unitario).
#[inline]
pub fn reject(v: Vector3, n: Vector3) -> Vector3 {
    v - n * v.dot(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vector3, b: Vector3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn identidad_no_cambia_nada() {
        let m = mul(&identity(), &translation(Vector3::new(1.0, 2.0, 3.0)));
        assert!(close(transform_point(&m, Vector3::zero()), Vector3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn rotacion_y_un_cuarto_de_vuelta() {
        let v = rotate_y(Vector3::new(0.0, 0.0, 1.0), std::f32::consts::FRAC_PI_2);
        assert!(close(v, Vector3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn rodrigues_coincide_con_rotate_y() {
        let v = Vector3::new(0.3, -0.5, 0.8);
        let a = 0.7;
        assert!(close(rotate_axis(v, Vector3::new(0.0, 1.0, 0.0), a), rotate_y(v, a)));
    }

    #[test]
    fn look_at_deja_el_objetivo_enfrente() {
        let view = look_at(Vector3::new(0.0, 0.0, 5.0), Vector3::zero(), Vector3::new(0.0, 1.0, 0.0));
        let p = transform_point(&view, Vector3::zero());
        // En la vista la cámara mira hacia -Z.
        assert!(close(p, Vector3::new(0.0, 0.0, -5.0)));
    }

    #[test]
    fn perspectiva_manda_near_y_far_a_menos_uno_y_uno() {
        let p = perspective(1.0, 1.0, 0.5, 100.0);
        let near = transform(&p, Vector3::new(0.0, 0.0, -0.5), 1.0);
        let far = transform(&p, Vector3::new(0.0, 0.0, -100.0), 1.0);
        assert!((near[2] / near[3] + 1.0).abs() < 1e-4);
        assert!((far[2] / far[3] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn from_basis_conserva_la_orientacion() {
        // Con la base del mundo tiene que salir lo mismo que una traslación + escala.
        let m = from_basis(Vector3::new(1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, 0.0, 1.0), 2.0);
        assert!(close(transform_point(&m, Vector3::new(0.0, 1.0, 0.0)), Vector3::new(1.0, 2.0, 0.0)));
    }
}
