use std::ops::{Add, AddAssign, Deref, DerefMut, Sub};

use num::{BigUint, CheckedSub};

use crate::Balance;

impl Deref for Balance {
    type Target = BigUint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Balance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Add<Balance> for Balance {
    type Output = Self;

    fn add(self, rhs: Balance) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Add<&'_ Balance> for &'_ Balance {
    type Output = Balance;

    fn add(self, rhs: &'_ Balance) -> Self::Output {
        Balance(&self.0 + &rhs.0)
    }
}

impl Add<Balance> for &'_ Balance {
    type Output = Balance;

    fn add(self, rhs: Balance) -> Self::Output {
        Balance(&self.0 + rhs.0)
    }
}

impl AddAssign<Balance> for Balance {
    fn add_assign(&mut self, rhs: Balance) {
        self.0 += rhs.0;
    }
}

impl AddAssign<&'_ Balance> for Balance {
    fn add_assign(&mut self, rhs: &Balance) {
        self.0 += &rhs.0;
    }
}

impl Sub<Balance> for Balance {
    type Output = Option<Self>;

    fn sub(self, rhs: Balance) -> Self::Output {
        self.0.checked_sub(&rhs.0).map(Self)
    }
}

impl Sub<&'_ Balance> for Balance {
    type Output = Option<Self>;

    fn sub(self, rhs: &Balance) -> Self::Output {
        self.0.checked_sub(&rhs.0).map(Self)
    }
}
