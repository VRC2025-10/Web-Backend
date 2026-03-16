use std::marker::PhantomData;

/// Marker trait for authorization levels.
/// Sealed — only our marker types implement it.
pub trait Role: Send + Sync + 'static {
    const LEVEL: u8;
    const NAME: &'static str;
}

// Zero-sized marker types — no runtime cost
pub struct Member;
pub struct Staff;
pub struct Admin;
pub struct SuperAdmin;

impl Role for Member {
    const LEVEL: u8 = 0;
    const NAME: &'static str = "member";
}
impl Role for Staff {
    const LEVEL: u8 = 1;
    const NAME: &'static str = "staff";
}
impl Role for Admin {
    const LEVEL: u8 = 2;
    const NAME: &'static str = "admin";
}
impl Role for SuperAdmin {
    const LEVEL: u8 = 3;
    const NAME: &'static str = "super_admin";
}

/// Compile-time role satisfaction check.
/// R satisfies minimum role Min iff R::LEVEL >= Min::LEVEL.
pub trait SatisfiesRole<Min: Role>: Role {}

// Implement role hierarchy
impl SatisfiesRole<Member> for Member {}
impl SatisfiesRole<Member> for Staff {}
impl SatisfiesRole<Member> for Admin {}
impl SatisfiesRole<Member> for SuperAdmin {}
impl SatisfiesRole<Staff> for Staff {}
impl SatisfiesRole<Staff> for Admin {}
impl SatisfiesRole<Staff> for SuperAdmin {}
impl SatisfiesRole<Admin> for Admin {}
impl SatisfiesRole<Admin> for SuperAdmin {}
impl SatisfiesRole<SuperAdmin> for SuperAdmin {}

/// Type-safe wrapper to carry the role phantom type.
/// Used internally by AuthenticatedUser.
pub struct RolePhantom<R: Role>(PhantomData<R>);

impl<R: Role> RolePhantom<R> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<R: Role> Default for RolePhantom<R> {
    fn default() -> Self {
        Self::new()
    }
}
