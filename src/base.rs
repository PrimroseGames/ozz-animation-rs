//!
//! Base types, traits and utils.
//!

use std::cell::{Ref, RefCell, RefMut};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::BuildHasher;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use thiserror::Error;

/// Ozz error type.
#[derive(Error, Debug)]
pub enum OzzError {
    /// Lock poisoned, only happens when using `Arc<RWLock<T>>` as `OzzBuf<T>`.
    #[error("Lock poisoned")]
    LockPoison,
    /// Validates job failed.
    #[error("Invalid job")]
    InvalidJob,
    /// Invalid buffer index.
    #[error("Invalid index")]
    InvalidIndex,

    /// Std io errors.
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    /// Std string errors.
    #[error("Utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// Read ozz archive tag error.
    #[error("Invalid tag")]
    InvalidTag,
    /// Read ozz archive version error.
    #[error("Invalid version")]
    InvalidVersion,

    /// Custom errors.
    /// Ozz-animation-rs does not generate this error (except test & nodejs), but you can use it in your own code.
    #[error("Custom error: {0}")]
    Custom(String),
}

impl OzzError {
    pub fn is_lock_poison(&self) -> bool {
        return match self {
            OzzError::LockPoison => true,
            _ => false,
        };
    }

    pub fn is_invalid_job(&self) -> bool {
        return match self {
            OzzError::InvalidJob => true,
            _ => false,
        };
    }

    pub fn is_io(&self) -> bool {
        return match self {
            OzzError::IO(_) => true,
            _ => false,
        };
    }

    pub fn is_utf8(&self) -> bool {
        return match self {
            OzzError::Utf8(_) => true,
            _ => false,
        };
    }

    pub fn is_invalid_tag(&self) -> bool {
        return match self {
            OzzError::InvalidTag => true,
            _ => false,
        };
    }

    pub fn is_invalid_version(&self) -> bool {
        return match self {
            OzzError::InvalidVersion => true,
            _ => false,
        };
    }

    pub fn is_custom(&self) -> bool {
        return match self {
            OzzError::Custom(_) => true,
            _ => false,
        };
    }
}

/// Defines the maximum number of joints.
/// This is limited in order to control the number of bits required to store
/// a joint index. Limiting the number of joints also helps handling worst
/// size cases, like when it is required to allocate an array of joints on
/// the stack.
pub const SKELETON_MAX_JOINTS: i32 = 1024;

/// Defines the maximum number of SoA elements required to store the maximum
/// number of joints.
pub const SKELETON_MAX_SOA_JOINTS: i32 = (SKELETON_MAX_JOINTS + 3) / 4;

/// Defines the index of the parent of the root joint (which has no parent in fact)
pub const SKELETON_NO_PARENT: i32 = -1;

/// A hasher builder that creates `DefaultHasher` with default keys.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicState;

impl DeterministicState {
    /// Creates a new `DeterministicState` that builds `DefaultHasher` with default keys.
    pub const fn new() -> DeterministicState {
        return DeterministicState;
    }
}

impl BuildHasher for DeterministicState {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> DefaultHasher {
        return DefaultHasher::default();
    }
}

/// Allow usize/i32/i16 use as ozz index.
pub trait OzzIndex {
    fn usize(&self) -> usize;
    fn i32(&self) -> i32;
}

macro_rules! ozz_index {
    ($type:ty) => {
        impl OzzIndex for $type {
            #[inline(always)]
            fn usize(&self) -> usize {
                return *self as usize;
            }

            #[inline(always)]
            fn i32(&self) -> i32 {
                return *self as i32;
            }
        }
    };
}

ozz_index!(usize);
ozz_index!(i32);
ozz_index!(i16);

/// Represents a reference to the ozz resource object.
/// `T` usually is `Skeleton` or `Animation`.
///
/// We use `OzzObj` to support `T`, `&T`, `Rc<T>` and `Arc<T>` at same time.
/// Or you can implement this trait to support your own reference type.
pub trait OzzObj<T: Debug> {
    fn obj(&self) -> &T;
}

impl<T: Debug> OzzObj<T> for T {
    #[inline(always)]
    fn obj(&self) -> &T {
        return self;
    }
}

impl<'t, T: Debug> OzzObj<T> for &'t T {
    #[inline(always)]
    fn obj(&self) -> &T {
        return self;
    }
}

impl<T: Debug> OzzObj<T> for *const T {
    #[inline(always)]
    fn obj(&self) -> &T {
        return unsafe { &**self };
    }
}

impl<T: Debug> OzzObj<T> for Rc<T> {
    #[inline(always)]
    fn obj(&self) -> &T {
        return self.as_ref();
    }
}

impl<T: Debug> OzzObj<T> for Arc<T> {
    #[inline(always)]
    fn obj(&self) -> &T {
        return self.as_ref();
    }
}

/// Represents a reference to the ozz immutable buffers.
/// `T` usually is `SoaTransform`, `Mat4`, .etc.
///
/// We use `OzzBuf` to support `&[T]`, `Vec<T>`, `Rc<RefCell<Vec<T>>>`, `Arc<RwLock<Vec<T>>>` at same time.
/// Or you can implement this trait to support your own immutable buffer types.
pub trait OzzBuf<T: Debug + Clone> {
    type Buf<'t>: Deref<Target = [T]>
    where
        Self: 't;

    fn buf(&self) -> Result<Self::Buf<'_>, OzzError>;
}

// pub trait OzzAsSlice<'t, T: Debug + Clone> {
//     fn slice(&'t self) -> Result<&'t [T], OzzError>;
// }

/// Represents a reference to the ozz mutable buffers.
/// `T` usually is `SoaTransform`, `Mat4`, .etc.
///
/// We use `OzzBuf` to support `&mut [T]`, `Vec<T>`, `Rc<RefCell<Vec<T>>>`, `Arc<RwLock<Vec<T>>>` at same time.
/// Or you can implement this trait to support your own writable buffer types.
pub trait OzzMutBuf<T: Debug + Clone>
where
    Self: OzzBuf<T>,
{
    type MutBuf<'t>: DerefMut<Target = [T]>
    where
        Self: 't;

    fn mut_buf(&mut self) -> Result<Self::MutBuf<'_>, OzzError>;
}

//
// &[T]
//

impl<'a, T: 'static + Debug + Clone> OzzBuf<T> for &'a [T] {
    type Buf<'b> = ObSliceRef<'b, T>
    where
        'a: 'b;

    #[inline(always)]
    fn buf(&self) -> Result<ObSliceRef<T>, OzzError> {
        return Ok(ObSliceRef(self));
    }
}

pub struct ObSliceRef<'t, T>(&'t [T]);

impl<'t, T> Deref for ObSliceRef<'t, T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        return self.0;
    }
}

//
// &mut [T]
//

impl<'a, T: 'static + Debug + Clone> OzzBuf<T> for &'a mut [T] {
    type Buf<'b> = ObSliceRef<'b, T>
    where
        'a: 'b;

    #[inline(always)]
    fn buf(&self) -> Result<ObSliceRef<T>, OzzError> {
        return Ok(ObSliceRef(self));
    }
}

impl<'a, T: 'static + Debug + Clone> OzzMutBuf<T> for &'a mut [T] {
    type MutBuf<'b> = ObSliceRefMut<'b, T>
    where
        'a: 'b;

    #[inline(always)]
    fn mut_buf(&mut self) -> Result<ObSliceRefMut<T>, OzzError> {
        return Ok(ObSliceRefMut(self));
    }
}

pub struct ObSliceRefMut<'t, T>(&'t mut [T]);

impl<'t, T> Deref for ObSliceRefMut<'t, T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        return self.0;
    }
}

impl<'t, T> DerefMut for ObSliceRefMut<'t, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        return self.0;
    }
}

//
// Vec<T>
//

impl<T: 'static + Debug + Clone> OzzBuf<T> for Vec<T> {
    type Buf<'t> = ObSliceRef<'t, T>;

    #[inline(always)]
    fn buf(&self) -> Result<ObSliceRef<T>, OzzError> {
        return Ok(ObSliceRef(self.as_slice()));
    }
}

impl<T: 'static + Debug + Clone> OzzMutBuf<T> for Vec<T> {
    type MutBuf<'t> = ObSliceRefMut<'t, T>;

    #[inline(always)]
    fn mut_buf(&mut self) -> Result<ObSliceRefMut<T>, OzzError> {
        return Ok(ObSliceRefMut(self));
    }
}

//
// Rc<RefCell<Vec<T>>>
//

impl<T: 'static + Debug + Clone> OzzBuf<T> for Rc<RefCell<Vec<T>>> {
    type Buf<'t> = ObCellRef<'t, T>;

    #[inline(always)]
    fn buf(&self) -> Result<ObCellRef<T>, OzzError> {
        return Ok(ObCellRef(self.borrow()));
    }
}

pub struct ObCellRef<'t, T>(Ref<'t, Vec<T>>);

impl<'t, T> Deref for ObCellRef<'t, T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        return self.0.as_slice();
    }
}

impl<T: 'static + Debug + Clone> OzzMutBuf<T> for Rc<RefCell<Vec<T>>> {
    type MutBuf<'t> = ObCellRefMut<'t, T>;

    #[inline(always)]
    fn mut_buf(&mut self) -> Result<ObCellRefMut<T>, OzzError> {
        return Ok(ObCellRefMut(self.borrow_mut()));
    }
}

pub struct ObCellRefMut<'t, T>(RefMut<'t, Vec<T>>);

impl<'t, T> Deref for ObCellRefMut<'t, T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        return self.0.as_slice();
    }
}

impl<'t, T> DerefMut for ObCellRefMut<'t, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        return self.0.as_mut_slice();
    }
}

//
// Arc<RwLock<Vec<T>>>
//

impl<T: 'static + Debug + Clone> OzzBuf<T> for Arc<RwLock<Vec<T>>> {
    type Buf<'t> = ObRwLockReadGuard<'t, T>;

    #[inline(always)]
    fn buf(&self) -> Result<ObRwLockReadGuard<T>, OzzError> {
        return match self.read() {
            Ok(guard) => Ok(ObRwLockReadGuard(guard)),
            Err(_) => Err(OzzError::LockPoison),
        };
    }
}

pub struct ObRwLockReadGuard<'t, T>(RwLockReadGuard<'t, Vec<T>>);

impl<'t, T> Deref for ObRwLockReadGuard<'t, T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        return self.0.as_slice();
    }
}

impl<T: 'static + Debug + Clone> OzzMutBuf<T> for Arc<RwLock<Vec<T>>> {
    type MutBuf<'t> = ObRwLockWriteGuard<'t, T>;

    #[inline(always)]
    fn mut_buf(&mut self) -> Result<ObRwLockWriteGuard<T>, OzzError> {
        return match self.write() {
            Ok(guard) => Ok(ObRwLockWriteGuard(guard)),
            Err(_) => Err(OzzError::LockPoison),
        };
    }
}

pub struct ObRwLockWriteGuard<'t, T>(RwLockWriteGuard<'t, Vec<T>>);

impl<'t, T> Deref for ObRwLockWriteGuard<'t, T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        return self.0.as_slice();
    }
}

impl<'t, T> DerefMut for ObRwLockWriteGuard<'t, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        return self.0.as_mut_slice();
    }
}
