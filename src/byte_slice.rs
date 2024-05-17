// Copyright 2024 The Fuchsia Authors
//
// Licensed under a BSD-style license <LICENSE-BSD>, Apache License, Version 2.0
// <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0>, or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed except according to
// those terms.

//! Traits for types that encapsulate a `[u8]`.
//!
//! These traits are used to bound the `B` parameter of [`Ref`].

use core::{
    cell,
    ops::{Deref, DerefMut},
};

#[cfg(doc)]
use crate::Ref;

// For each trait polyfill, as soon as the corresponding feature is stable, the
// polyfill import will be unused because method/function resolution will prefer
// the inherent method/function over a trait method/function. Thus, we suppress
// the `unused_imports` warning.
//
// See the documentation on `util::polyfills` for more information.
#[allow(unused_imports)]
use crate::util::polyfills::{self, NonNullExt as _, NumExt as _};

/// A mutable or immutable reference to a byte slice.
///
/// `ByteSlice` abstracts over the mutability of a byte slice reference, and is
/// implemented for various special reference types such as [`Ref<[u8]>`] and
/// [`RefMut<[u8]>`].
///
/// [`Ref<[u8]>`]: core::cell::Ref
/// [`RefMut<[u8]>`]: core::cell::RefMut
///
/// # Safety
///
/// Implementations of `ByteSlice` must promise that their implementations of
/// [`Deref`] and [`DerefMut`] are "stable". In particular, given `B: ByteSlice`
/// and `b: B`, two calls, each to either `b.deref()` or `b.deref_mut()`, must
/// return a byte slice with the same address and length. This must hold even if
/// the two calls are separated by an arbitrary sequence of calls to methods on
/// `ByteSlice`, [`ByteSliceMut`], [`IntoByteSlice`], or [`IntoByteSliceMut`],
/// or on their super-traits. This does *not* need to hold if the two calls are
/// separated by any method calls, field accesses, or field modifications *other
/// than* those from these traits.
///
/// Note that this also implies that, given `b: B`, the address and length
/// cannot be modified via objects other than `b`, either on the same thread or
/// on another thread.
pub unsafe trait ByteSlice: Deref<Target = [u8]> + Sized {}

/// A mutable reference to a byte slice.
///
/// `ByteSliceMut` abstracts over various ways of storing a mutable reference to
/// a byte slice, and is implemented for various special reference types such as
/// `RefMut<[u8]>`.
///
/// `ByteSliceMut` is a shorthand for [`ByteSlice`] and [`DerefMut`].
pub trait ByteSliceMut: ByteSlice + DerefMut {}
impl<B: ByteSlice + DerefMut> ByteSliceMut for B {}

/// A [`ByteSlice`] which can be copied without violating dereference stability.
///
/// # Safety
///
/// If `B: CopyableByteSlice`, then the dereference stability properties
/// required by `ByteSlice` (see that trait's safety documentation) do not only
/// hold regarding two calls to `b.deref()` or `b.deref_mut()`, but also hold
/// regarding `c.deref()` or `c.deref_mut()`, where `c` is produced by copying
/// `b`.
pub unsafe trait CopyableByteSlice: ByteSlice + Copy + CloneableByteSlice {}

/// A [`ByteSlice`] which can be cloned without violating dereference stability.
///
/// # Safety
///
/// If `B: CloneableByteSlice`, then the dereference stability properties
/// required by `ByteSlice` (see that trait's safety documentation) do not only
/// hold regarding two calls to `b.deref()` or `b.deref_mut()`, but also hold
/// regarding `c.deref()` or `c.deref_mut()`, where `c` is produced by
/// `b.clone()`, `b.clone().clone()`, etc.
pub unsafe trait CloneableByteSlice: ByteSlice + Clone {}

/// A [`ByteSlice`] that can be split in two.
///
/// # Safety
///
/// Unsafe code may depend for its soundness on the assumption that `split_at`
/// and `split_at_unchecked` are implemented correctly. In particular, given `B:
/// SplitByteSlice` and `b: B`, if `b.deref()` returns a byte slice with address
/// `addr` and length `len`, then if `split <= len`, both of these
/// invocations:
/// - `b.split_at(split)`
/// - `b.split_at_unchecked(split)`
///
/// ...will return `(first, second)` such that:
/// - `first`'s address is `addr` and its length is `split`
/// - `second`'s address is `addr + split` and its length is `len - split`
pub unsafe trait SplitByteSlice: ByteSlice {
    /// Splits the slice at the midpoint.
    ///
    /// `x.split_at(mid)` returns `x[..mid]` and `x[mid..]`.
    ///
    /// # Panics
    ///
    /// `x.split_at(mid)` panics if `mid > x.deref().len()`.
    #[must_use]
    #[inline]
    fn split_at(self, mid: usize) -> (Self, Self) {
        if let Ok(splits) = try_split_at(self, mid) {
            splits
        } else {
            panic!("mid > len")
        }
    }

    /// Splits the slice at the midpoint, possibly omitting bounds checks.
    ///
    /// `x.split_at_unchecked(mid)` returns `x[..mid]` and `x[mid..]`.
    ///
    /// # Safety
    ///
    /// `mid` must not be greater than `x.deref().len()`.
    #[must_use]
    unsafe fn split_at_unchecked(self, mid: usize) -> (Self, Self);
}

/// Attempts to split the slice at the midpoint.
///
/// `x.try_split_at(mid)` returns `Ok((x[..mid], x[mid..]))` if `mid <=
/// x.deref().len()` and otherwise returns `Err(x)`.
///
/// # Safety
///
/// Unsafe code may rely on this function correctly implementing the above
/// functionality.
#[inline]
pub(crate) fn try_split_at<S>(slice: S, mid: usize) -> Result<(S, S), S>
where
    S: SplitByteSlice,
{
    if mid <= slice.deref().len() {
        // SAFETY: Above, we ensure that `mid <= self.deref().len()`. By
        // invariant on `ByteSlice`, a supertrait of `SplitByteSlice`,
        // `.deref()` is guranteed to be "stable"; i.e., it will always
        // dereference to a byte slice of the same address and length. Thus, we
        // can be sure that the above precondition remains satisfied through the
        // call to `split_at_unchecked`.
        unsafe { Ok(slice.split_at_unchecked(mid)) }
    } else {
        Err(slice)
    }
}

/// A shorthand for [`SplitByteSlice`] and [`ByteSliceMut`].
pub trait SplitByteSliceMut: SplitByteSlice + ByteSliceMut {}
impl<B: SplitByteSlice + ByteSliceMut> SplitByteSliceMut for B {}

/// A [`ByteSlice`] that conveys no ownership, and so can be converted into a
/// byte slice.
///
/// Some `ByteSlice` types (notably, the standard library's [`Ref`] type) convey
/// ownership, and so they cannot soundly be moved by-value into a byte slice
/// type (`&[u8]`). Some methods in this crate's API (such as [`Ref::into_ref`])
/// are only compatible with `ByteSlice` types without these ownership
/// semantics.
///
/// # Safety
///
/// Invoking `self.into()` produces a `&[u8]` with identical address and length
/// as the slice produced by `self.deref()`. Note that this implies that the
/// slice produced by `self.into()` is "stable" in the same sense as defined by
/// [`ByteSlice`]'s safety invariant.
///
/// If `Self: Into<&mut [u8]>`, then the same stability requirement holds of
/// `<Self as Into<&mut [u8]>>::into`.
///
/// [`Ref`]: core::cell::Ref
pub unsafe trait IntoByteSlice<'a>: ByteSlice + Into<&'a [u8]> {}

/// A [`ByteSliceMut`] that conveys no ownership, and so can be converted into a
/// mutable byte slice.
///
/// Some `ByteSliceMut` types (notably, the standard library's [`RefMut`] type)
/// convey ownership, and so they cannot soundly be moved by-value into a byte
/// slice type (`&mut [u8]`). Some methods in this crate's API (such as
/// [`Ref::into_mut`]) are only compatible with `ByteSliceMut` types without
/// these ownership semantics.
///
/// [`RefMut`]: core::cell::RefMut
pub trait IntoByteSliceMut<'a>: ByteSliceMut + Into<&'a mut [u8]> {}
impl<'a, B: ByteSliceMut + Into<&'a mut [u8]>> IntoByteSliceMut<'a> for B {}

// TODO(#429): Add a "SAFETY" comment and remove this `allow`.
#[allow(clippy::undocumented_unsafe_blocks)]
unsafe impl<'a> ByteSlice for &'a [u8] {}

// TODO(#429): Add a "SAFETY" comment and remove this `allow`.
#[allow(clippy::undocumented_unsafe_blocks)]
unsafe impl<'a> CopyableByteSlice for &'a [u8] {}

// TODO(#429): Add a "SAFETY" comment and remove this `allow`.
#[allow(clippy::undocumented_unsafe_blocks)]
unsafe impl<'a> CloneableByteSlice for &'a [u8] {}

// SAFETY: This delegates to `polyfills:split_at_unchecked`, which is documented
// to correctly split `self` into two slices at the given `mid` point.
unsafe impl<'a> SplitByteSlice for &'a [u8] {
    #[inline]
    unsafe fn split_at_unchecked(self, mid: usize) -> (Self, Self) {
        // SAFETY: By contract on caller, `mid` is not greater than
        // `bytes.len()`.
        unsafe { (<[u8]>::get_unchecked(self, ..mid), <[u8]>::get_unchecked(self, mid..)) }
    }
}

// SAFETY: The standard library impl of `From<T> for T` [1] cannot be removed
// without being a backwards-breaking change. Thus, we can rely on it continuing
// to exist. So long as that is the impl backing `Into<&[u8]> for &[u8]`, we
// know (barring a truly ridiculous stdlib implementation) that this impl is
// simply implemented as `fn into(bytes: &[u8]) -> &[u8] { bytes }` (or
// something semantically equivalent to it). Thus, `ByteSlice`'s stability
// guarantees are not violated by the `Into<&[u8]>` impl.
//
// [1] https://doc.rust-lang.org/std/convert/trait.From.html#impl-From%3CT%3E-for-T
unsafe impl<'a> IntoByteSlice<'a> for &'a [u8] {}

// TODO(#429): Add a "SAFETY" comment and remove this `allow`.
#[allow(clippy::undocumented_unsafe_blocks)]
unsafe impl<'a> ByteSlice for &'a mut [u8] {}

// SAFETY: This delegates to `polyfills:split_at_mut_unchecked`, which is
// documented to correctly split `self` into two slices at the given `mid`
// point.
unsafe impl<'a> SplitByteSlice for &'a mut [u8] {
    #[inline]
    unsafe fn split_at_unchecked(self, mid: usize) -> (Self, Self) {
        use core::slice::from_raw_parts_mut;

        // `l_ptr` is non-null, because `self` is non-null, by invariant on
        // `&mut [u8]`.
        let l_ptr = self.as_mut_ptr();

        // SAFETY: By contract on caller, `mid` is not greater than
        // `self.len()`.
        let r_ptr = unsafe { l_ptr.add(mid) };

        let l_len = mid;

        // SAFETY: By contract on caller, `mid` is not greater than
        // `self.len()`.
        //
        // TODO(#67): Remove this allow. See NumExt for more details.
        #[allow(unstable_name_collisions, clippy::incompatible_msrv)]
        let r_len = unsafe { self.len().unchecked_sub(mid) };

        // SAFETY: These invocations of `from_raw_parts_mut` satisfy its
        // documented safety preconditions [1]:
        // - The data `l_ptr` and `r_ptr` are valid for both reads and writes of
        //   `l_len` and `r_len` bytes, respectively, and they are trivially
        //   aligned. In particular:
        //   - The entire memory range of each slice is contained within a
        //     single allocated object, since `l_ptr` and `r_ptr` are both
        //     derived from within the address range of `self`.
        //   - Both `l_ptr` and `r_ptr` are non-null and trivially aligned.
        //     `self` is non-null by invariant on `&mut [u8]`, and the
        //     operations that derive `l_ptr` and `r_ptr` from `self` do not
        //     nullify either pointer.
        // - The data `l_ptr` and `r_ptr` point to `l_len` and `r_len`,
        //   respectively, consecutive properly initialized values of type `u8`.
        //   This is true for `self` by invariant on `&mut [u8]`, and remains
        //   true for these two sub-slices of `self`.
        // - The memory referenced by the returned slice cannot be accessed
        //   through any other pointer (not derived from the return value) for
        //   the duration of lifetime `'a``, because:
        //   - `split_at_unchecked` consumes `self` (which is not `Copy`),
        //   - `split_at_unchecked` does not exfiltrate any references to this
        //     memory, besides those references returned below,
        //   - the returned slices are non-overlapping.
        // - The individual sizes of the sub-slices of `self` are no larger than
        //   `isize::MAX`, because their combined sizes are no larger than
        //   `isize::MAX`, by invariant on `self`.
        //
        // [1] https://doc.rust-lang.org/std/slice/fn.from_raw_parts_mut.html#safety
        unsafe { (from_raw_parts_mut(l_ptr, l_len), from_raw_parts_mut(r_ptr, r_len)) }
    }
}

// TODO(#429): Add a "SAFETY" comment and remove this `allow`.
#[allow(clippy::undocumented_unsafe_blocks)]
unsafe impl<'a> ByteSlice for cell::Ref<'a, [u8]> {}

// SAFETY: This delegates to stdlib implementation of `Ref::map_split`, which is
// assumed to be correct, and `SplitByteSlice::split_at_unchecked`, which is
// documented to correctly split `self` into two slices at the given `mid`
// point.
unsafe impl<'a> SplitByteSlice for cell::Ref<'a, [u8]> {
    #[inline]
    unsafe fn split_at_unchecked(self, mid: usize) -> (Self, Self) {
        cell::Ref::map_split(self, |slice|
            // SAFETY: By precondition on caller, `mid` is not greater than
            // `slice.len()`.
            unsafe {
                SplitByteSlice::split_at_unchecked(slice, mid)
            })
    }
}

// TODO(#429): Add a "SAFETY" comment and remove this `allow`.
#[allow(clippy::undocumented_unsafe_blocks)]
unsafe impl<'a> ByteSlice for cell::RefMut<'a, [u8]> {}

// SAFETY: This delegates to stdlib implementation of `RefMut::map_split`, which
// is assumed to be correct, and `SplitByteSlice::split_at_unchecked`, which is
// documented to correctly split `self` into two slices at the given `mid`
// point.
unsafe impl<'a> SplitByteSlice for cell::RefMut<'a, [u8]> {
    #[inline]
    unsafe fn split_at_unchecked(self, mid: usize) -> (Self, Self) {
        cell::RefMut::map_split(self, |slice|
            // SAFETY: By precondition on caller, `mid` is not greater than
            // `slice.len()`
            unsafe {
                SplitByteSlice::split_at_unchecked(slice, mid)
            })
    }
}
