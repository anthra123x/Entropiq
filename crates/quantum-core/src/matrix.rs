use crate::complex::Complex64;
use std::ops::{Add, Mul};

/// Stack-allocated, zero-heap N x N complex matrix for quantum state operators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CMatrix<const N: usize> {
    pub data: [[Complex64; N]; N],
}

impl<const N: usize> CMatrix<N> {
    #[inline(always)]
    pub const fn zeros() -> Self {
        Self {
            data: [[Complex64::ZERO; N]; N],
        }
    }

    #[inline(always)]
    pub const fn identity() -> Self {
        let mut mat = [[Complex64::ZERO; N]; N];
        let mut i = 0;
        while i < N {
            mat[i][i] = Complex64::ONE;
            i += 1;
        }
        Self { data: mat }
    }

    #[inline(always)]
    pub fn get(&self, r: usize, c: usize) -> Complex64 {
        self.data[r][c]
    }

    #[inline(always)]
    pub fn set(&mut self, r: usize, c: usize, val: Complex64) {
        self.data[r][c] = val;
    }

    /// Computes the matrix trace: sum of diagonal elements.
    #[inline(always)]
    pub fn trace(&self) -> Complex64 {
        let mut sum = Complex64::ZERO;
        for i in 0..N {
            sum = sum + self.data[i][i];
        }
        sum
    }

    /// Computes the conjugate transpose (Hermitian adjoint: M^†).
    #[inline(always)]
    pub fn conjugate_transpose(&self) -> Self {
        let mut res = [[Complex64::ZERO; N]; N];
        for r in 0..N {
            for c in 0..N {
                res[c][r] = self.data[r][c].conjugate();
            }
        }
        Self { data: res }
    }

    /// Validates if the matrix is Hermitian (M == M^† within tolerance).
    pub fn is_hermitian(&self, tol: f64) -> bool {
        for r in 0..N {
            if self.data[r][r].im.abs() > tol {
                return false;
            }
            for c in (r + 1)..N {
                let diff = self.data[r][c] - self.data[c][r].conjugate();
                if diff.norm() > tol {
                    return false;
                }
            }
        }
        true
    }

    /// Standard matrix multiplication (O(N^3), cache-friendly unrolled for small N).
    pub fn matmul(&self, other: &Self) -> Self {
        let mut res = [[Complex64::ZERO; N]; N];
        for i in 0..N {
            for k in 0..N {
                let a = self.data[i][k];
                for j in 0..N {
                    res[i][j] = res[i][j] + a * other.data[k][j];
                }
            }
        }
        Self { data: res }
    }

    /// Scalar multiplication.
    pub fn scale(&self, factor: Complex64) -> Self {
        let mut res = [[Complex64::ZERO; N]; N];
        for r in 0..N {
            for c in 0..N {
                res[r][c] = self.data[r][c] * factor;
            }
        }
        Self { data: res }
    }

    /// Matrix addition.
    pub fn add_mat(&self, other: &Self) -> Self {
        let mut res = [[Complex64::ZERO; N]; N];
        for r in 0..N {
            for c in 0..N {
                res[r][c] = self.data[r][c] + other.data[r][c];
            }
        }
        Self { data: res }
    }
}

impl CMatrix<2> {
    /// Fast closed-form eigenvalues for 2x2 Hermitian matrix.
    /// Returns sorted in descending order: [lambda_max, lambda_min].
    pub fn hermitian_eigenvalues_2x2(&self) -> [f64; 2] {
        let a = self.data[0][0].re;
        let d = self.data[1][1].re;
        let b_norm_sq = self.data[0][1].norm_sq();

        let half_diff = (a - d) * 0.5;
        let discr = (half_diff * half_diff + b_norm_sq).sqrt();
        let mid = (a + d) * 0.5;

        let lambda_1 = mid + discr;
        let lambda_2 = mid - discr;
        [lambda_1, lambda_2]
    }
}

impl<const N: usize> Add for CMatrix<N> {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        self.add_mat(&rhs)
    }
}

impl<const N: usize> Mul for CMatrix<N> {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        self.matmul(&rhs)
    }
}
