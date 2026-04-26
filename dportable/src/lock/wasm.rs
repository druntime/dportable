use std::{
    cell::RefCell,
    fmt::Display,
    ops::{Deref, DerefMut},
};

/// A mutual exclusion primitive useful for protecting shared data.
///
/// This mutex will block threads waiting for the lock to become available.
/// The mutex can be statically initialized or created by the `new` constructor.
/// Each mutex has a type parameter which represents the data that it is protecting.
/// The data can only be accessed through the RAII guards returned from `lock` and `try_lock`,
/// which guarantees that the data is only ever accessed when the mutex is locked.
#[derive(Debug)]
pub struct Mutex<T>(pub RefCell<T>)
where
    T: ?Sized;

/// An RAII implementation of a “scoped lock” of a mutex.
/// When this structure is dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through
/// this guard via its `Deref` and `DerefMut` implementations.
#[derive(Debug)]
pub struct MutexGuard<'a, T>(std::cell::RefMut<'a, T>)
where
    T: ?Sized + 'a;

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Display for MutexGuard<'_, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    pub fn new(value: T) -> Self {
        Mutex(RefCell::new(value))
    }

    /// Acquires a mutex, blocking the current thread until it is able to do so.
    ///
    /// This function will block the local thread until it is available to acquire
    /// the mutex. Upon returning, the thread is the only thread with the mutex held.
    /// An RAII guard is returned to allow scoped unlock of the lock.
    /// When the guard goes out of scope, the mutex will be unlocked.
    ///
    /// Attempts to lock a mutex in the thread which already holds the lock will result in a deadlock.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard(self.0.borrow_mut())
    }

    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then `None` is returned.
    /// Otherwise, an RAII guard is returned. The lock will be unlocked when the guard
    /// is dropped.
    ///
    /// This function does not block.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.0.try_borrow_mut().ok().map(MutexGuard)
    }
}

/// A reader-writer lock.
///
/// The RAII guards returned from the locking methods implement
/// `Deref` (and `DerefMut` for the write methods) to allow access
/// to the contained of the lock.
#[derive(Debug)]
pub struct RwLock<T>(pub RefCell<T>)
where
    T: ?Sized;

/// RAII structure used to release the shared read access of a lock when dropped.
#[derive(Debug)]
pub struct RwLockReadGuard<'a, T>(std::cell::Ref<'a, T>)
where
    T: ?Sized + 'a;

impl<T> Display for RwLockReadGuard<'_, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// RAII structure used to release the exclusive write access of a lock when dropped.
#[derive(Debug)]
pub struct RwLockWriteGuard<'a, T>(std::cell::RefMut<'a, T>)
where
    T: ?Sized + 'a;

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Display for RwLockWriteGuard<'_, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<T> RwLock<T> {
    /// Creates a new instance of an `RwLock<T>` which is unlocked.
    pub fn new(value: T) -> Self {
        RwLock(RefCell::new(value))
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread
    /// until it can be acquired.
    ///
    /// The calling thread will be blocked until there are no more writers
    /// which hold the lock. There may be other readers currently inside
    /// the lock when this method returns.
    ///
    /// Note that attempts to recursively acquire a read lock on a `RwLock`
    /// when the current thread already holds one may result in a deadlock.
    ///
    /// Returns an RAII guard which will release this thread’s shared access
    /// once it is dropped.
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        RwLockReadGuard(self.0.borrow())
    }

    /// Attempts to acquire this `RwLock` with shared read access.
    ///
    /// If the access could not be granted at this time, then `None` is returned.
    /// Otherwise, an RAII guard is returned which will release the shared access
    /// when it is dropped.
    ///
    /// This function does not block.
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        self.0.try_borrow().ok().map(RwLockReadGuard)
    }

    /// Locks this `RwLock` with exclusive write access, blocking the current thread
    /// until it can be acquired.
    ///
    /// This function will not return while other writers or other readers currently
    /// have access to the lock.
    ///
    /// Returns an RAII guard which will drop the write access of this RwLock when dropped.
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        RwLockWriteGuard(self.0.borrow_mut())
    }

    /// Attempts to lock this `RwLock` with exclusive write access.
    ///
    /// If the lock could not be acquired at this time, then `None` is returned.
    /// Otherwise, an RAII guard is returned which will release the lock when it is dropped.
    ///
    /// This function does not block.
    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        self.0.try_borrow_mut().ok().map(RwLockWriteGuard)
    }
}
