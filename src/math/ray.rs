// Clippy complains about normal mathematical symbols like A, B, C for quadratic equation.
#![allow(clippy::many_single_char_names)]

use crate::math::{
    plane::Plane,
    vec3::Vec3,
    is_point_inside_triangle,
    solve_quadratic,
};

pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Default for Ray {
    fn default() -> Self {
        Ray {
            origin: Vec3::default(),
            dir: Vec3::new(0.0, 0.0, 1.0),
        }
    }
}

/// Pair of ray equation parameters.
#[derive(Clone, Debug)]
pub struct IntersectionResult {
    pub min: f32,
    pub max: f32,
}

impl IntersectionResult {
    pub fn from_slice(roots: &[f32]) -> Self {
        let mut min = std::f32::MAX;
        let mut max = -std::f32::MAX;
        for n in roots {
            min = min.min(*n);
            max = max.max(*n);
        }
        Self {
            min,
            max,
        }
    }

    pub fn from_set(results: &[Option<IntersectionResult>]) -> Option<Self> {
        let mut result = None;
        for v in results {
            match result {
                None => result = v.clone(),
                Some(ref mut result) => {
                    if let Some(v) = v {
                        result.merge(v.min);
                        result.merge(v.max);
                    }
                }
            }
        }
        result
    }

    /// Updates min and max ray equation parameters according to a new parameter -
    /// expands range if `param` was outside of that range.
    pub fn merge(&mut self, param: f32) {
        if param < self.min {
            self.min = param;
        }
        if param > self.max {
            self.max = param;
        }
    }

    pub fn merge_slice(&mut self, params: &[f32]) {
        for param in params {
            self.merge(*param)
        }
    }
}

pub enum CylinderKind {
    Infinite,
    Finite,
    Capped,
}

impl Ray {
    /// Creates ray from two points. May fail if begin == end.
    #[inline]
    pub fn from_two_points(begin: &Vec3, end: &Vec3) -> Option<Ray> {
        let dir = *end - *begin;
        if dir.len() >= std::f32::EPSILON {
            Some(Ray { origin: *begin, dir })
        } else {
            None
        }
    }

    /// Checks intersection with sphere. Returns two intersection points or none
    /// if there was no intersection.
    #[inline]
    pub fn sphere_intersection_points(&self, position: &Vec3, radius: f32) -> Option<[Vec3; 2]> {
        self.try_eval_points(self.sphere_intersection(position, radius))
    }

    pub fn sphere_intersection(&self, position: &Vec3, radius: f32) -> Option<IntersectionResult> {
        let d = self.origin - *position;
        let a = self.dir.dot(&self.dir);
        let b = 2.0 * self.dir.dot(&d);
        let c = d.dot(&d) - radius * radius;
        if let Some(roots) = solve_quadratic(a, b, c) {
            Some(IntersectionResult::from_slice(&roots))
        } else {
            None
        }
    }

    /// Checks intersection with sphere.
    #[inline]
    pub fn is_intersect_sphere(&self, position: Vec3, radius: f32) -> bool {
        let d = self.origin - position;
        let a = self.dir.dot(&self.dir);
        let b = 2.0 * self.dir.dot(&d);
        let c = d.dot(&d) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;
        discriminant >= 0.0
    }

    /// Returns t factor (at pt=o+d*t equation) for projection of given point at ray
    #[inline]
    pub fn project_point(&self, point: Vec3) -> f32 {
        (point - self.origin).dot(&self.dir) / self.dir.sqr_len()
    }

    /// Returns point on ray which defined by pt=o+d*t equation.
    #[inline]
    pub fn get_point(&self, t: f32) -> Vec3 {
        self.origin + self.dir.scale(t)
    }

    pub fn box_intersection(&self, min: &Vec3, max: &Vec3) -> Option<IntersectionResult> {
        let (mut tmin, mut tmax) = if self.dir.x >= 0.0 {
            ((min.x - self.origin.x) / self.dir.x,
             (max.x - self.origin.x) / self.dir.x)
        } else {
            ((max.x - self.origin.x) / self.dir.x,
             (min.x - self.origin.x) / self.dir.x)
        };

        let (tymin, tymax) = if self.dir.y >= 0.0 {
            ((min.y - self.origin.y) / self.dir.y,
             (max.y - self.origin.y) / self.dir.y)
        } else {
            ((max.y - self.origin.y) / self.dir.y,
             (min.y - self.origin.y) / self.dir.y)
        };

        if tmin > tymax || (tymin > tmax) {
            return None;
        }
        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }
        let (tzmin, tzmax) = if self.dir.z >= 0.0 {
            ((min.z - self.origin.z) / self.dir.z,
             (max.z - self.origin.z) / self.dir.z)
        } else {
            ((max.z - self.origin.z) / self.dir.z,
             (min.z - self.origin.z) / self.dir.z)
        };

        if (tmin > tzmax) || (tzmin > tmax) {
            return None;
        }
        if tzmin > tmin {
            tmin = tzmin;
        }
        if tzmax < tmax {
            tmax = tzmax;
        }
        if tmin < 1.0 && tmax > 0.0 {
            Some(IntersectionResult {
                min: tmin,
                max: tmax,
            })
        } else {
            None
        }
    }

    pub fn box_intersection_points(&self, min: &Vec3, max: &Vec3) -> Option<[Vec3; 2]> {
        self.try_eval_points(self.box_intersection(min, max))
    }

    /// Solves plane equation in order to find ray equation parameter.
    /// There is no intersection if result < 0.
    pub fn plane_intersection(&self, plane: &Plane) -> f32 {
        let u = -(self.origin.dot(&plane.normal) + plane.d);
        let v = self.dir.dot(&plane.normal);
        u / v
    }

    pub fn plane_intersection_point(&self, plane: &Plane) -> Option<Vec3> {
        let t = self.plane_intersection(plane);
        if t < 0.0 || t > 1.0 {
            None
        } else {
            Some(self.get_point(t))
        }
    }

    pub fn triangle_intersection(&self, vertices: &[Vec3; 3]) -> Option<Vec3> {
        let ba = vertices[1] - vertices[0];
        let ca = vertices[2] - vertices[0];
        let plane = Plane::from_normal_and_point(&ba.cross(&ca), &vertices[0]).ok()?;

        if let Some(point) = self.plane_intersection_point(&plane) {
            if is_point_inside_triangle(&point, vertices) {
                return Some(point);
            }
        }
        None
    }

    /// Generic ray-cylinder intersection test.
    ///
    /// https://mrl.nyu.edu/~dzorin/rend05/lecture2.pdf
    ///
    ///  Infinite cylinder oriented along line pa + va * t:
    ///      sqr_len(q - pa - dot(va, q - pa) * va) - r ^ 2 = 0
    ///  where q - point on cylinder, substitute q with ray p + v * t:
    ///     sqr_len(p - pa + vt - dot(va, p - pa + vt) * va) - r ^ 2 = 0
    ///  reduce to A * t * t + B * t + C = 0 (quadratic equation), where:
    ///     A = sqr_len(v - dot(v, va) * va)
    ///     B = 2 * dot(v - dot(v, va) * va, dp - dot(dp, va) * va)
    ///     C = sqr_len(dp - dot(dp, va) * va) - r ^ 2
    ///     where dp = p - pa
    ///  to find intersection points we have to solve quadratic equation
    ///  to get root which will be t parameter of ray equation.
    pub fn cylinder_intersection(&self, pa: &Vec3, pb: &Vec3, r: f32, kind: CylinderKind) -> Option<IntersectionResult> {
        let va = (*pb - *pa).normalized().unwrap_or_else(|| Vec3::new(0.0, 1.0, 0.0));
        let vl = self.dir - va.scale(self.dir.dot(&va));
        let dp = self.origin - *pa;
        let dpva = dp - va.scale(dp.dot(&va));

        let a = vl.sqr_len();
        let b = 2.0 * vl.dot(&dpva);
        let c = dpva.sqr_len() - r * r;

        // Get roots for cylinder surfaces
        if let Some(cylinder_roots) = solve_quadratic(a, b, c) {
            match kind {
                CylinderKind::Infinite => Some(IntersectionResult::from_slice(&cylinder_roots)),
                CylinderKind::Capped => {
                    let mut result = IntersectionResult::from_slice(&cylinder_roots);
                    // In case of cylinder with caps we have to check intersection with caps
                    for (cap_center, cap_normal) in [(pa, -va), (pb, va)].iter() {
                        let cap_plane = Plane::from_normal_and_point(cap_normal, cap_center).unwrap();
                        let t = self.plane_intersection(&cap_plane);
                        if t > 0.0 {
                            let intersection = self.get_point(t);
                            if cap_center.sqr_distance(&intersection) <= r * r {
                                // Point inside cap bounds
                                result.merge(t);
                            }
                        }
                    }
                    result.merge_slice(&cylinder_roots);
                    Some(result)
                }
                CylinderKind::Finite => {
                    // In case of finite cylinder without caps we have to check that intersection
                    // points on cylinder surface are between two planes of caps.
                    let mut result = None;
                    for root in cylinder_roots.iter() {
                        let int_point = self.get_point(*root);
                        if (int_point - *pa).dot(&va) >= 0.0 && (*pb - int_point).dot(&va) >= 0.0 {
                            match &mut result {
                                None => result = Some(IntersectionResult { min: *root, max: *root }),
                                Some(result) => result.merge(*root),
                            }
                        }
                    }
                    result
                }
            }
        } else {
            // We have no roots, so no intersection.
            None
        }
    }

    pub fn try_eval_points(&self, result: Option<IntersectionResult>) -> Option<[Vec3; 2]> {
        match result {
            None => None,
            Some(result) => {
                let a = if result.min >= 0.0 && result.min <= 1.0 {
                    Some(self.get_point(result.min))
                } else {
                    None
                };

                let b = if result.max >= 0.0 && result.max <= 1.0 {
                    Some(self.get_point(result.max))
                } else {
                    None
                };

                match a {
                    None => match b {
                        None => None,
                        Some(b) => Some([b, b]),
                    }
                    Some(a) => match b {
                        None => Some([a, a]),
                        Some(b) => Some([a, b])
                    }
                }
            }
        }
    }

    pub fn capsule_intersection(&self, pa: &Vec3, pb: &Vec3, radius: f32) -> Option<[Vec3; 2]> {
        // Dumb approach - check intersection with finite cylinder without caps,
        // then check two sphere caps.
        let cylinder = self.cylinder_intersection(pa, pb, radius, CylinderKind::Finite);
        let cap_a = self.sphere_intersection(pa, radius);
        let cap_b = self.sphere_intersection(pb, radius);
        self.try_eval_points(IntersectionResult::from_set(&[cylinder, cap_a, cap_b]))
    }
}

#[cfg(test)]
mod test {
    use crate::math::ray::Ray;
    use crate::math::vec3::Vec3;

    #[test]
    fn intersection() {
        let triangle = [Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0)];
        let ray = Ray::from_two_points(&Vec3::new(0.0, 0.0, -2.0),
                                       &Vec3::new(0.0, 0.0, -1.0)).unwrap();
        assert!(ray.triangle_intersection(&triangle).is_none());
    }
}