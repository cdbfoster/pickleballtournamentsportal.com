//! Inspired by the owning_ref crate, a GuardStack is reference to a value that lies underneath a chain of guards.
//! Important to use this only on values that will have a stable address over the course of the lock, like mutex-guarded, or heap-allocated values.

use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;

pub struct GuardStack<'a, G, T>
where
    T: ?Sized,
{
    guards: G,
    value: SendPtr<T>,
    marker: PhantomData<&'a T>,
}

#[allow(dead_code)]
impl<'a, G, T> GuardStack<'a, G, T>
where
    T: ?Sized,
{
    pub fn new(guard: G) -> GuardStack<'a, G, T>
    where
        G: Deref<Target = T>,
    {
        GuardStack {
            value: SendPtr(&*guard),
            guards: guard,
            marker: PhantomData,
        }
    }

    pub fn map<F, U, V>(self, f: F) -> GuardStack<'a, G, U>
    where
        F: FnOnce(&'a T) -> V,
        U: 'a + ?Sized,
        V: Deref<Target = U>,
    {
        GuardStack {
            value: SendPtr(&*f(unsafe { std::mem::transmute(&*self.value) })),
            guards: self.guards,
            marker: PhantomData,
        }
    }

    pub fn try_map<F, U, V, E>(self, f: F) -> Result<GuardStack<'a, G, U>, E>
    where
        F: FnOnce(&'a T) -> Result<V, E>,
        U: 'a + ?Sized,
        V: Deref<Target = U>,
    {
        Ok(GuardStack {
            value: SendPtr(&*f(unsafe { std::mem::transmute(&*self.value) })?),
            guards: self.guards,
            marker: PhantomData,
        })
    }

    pub fn push_guard<F, H, U>(self, f: F) -> GuardStack<'a, (G, H), U>
    where
        F: FnOnce(&'a T) -> H,
        H: Deref<Target = U>,
        U: 'a + ?Sized,
    {
        let new_guard = f(unsafe { std::mem::transmute(&*self.value) });
        GuardStack {
            value: SendPtr(&*new_guard),
            guards: (self.guards, new_guard),
            marker: PhantomData,
        }
    }

    pub fn try_push_guard<F, H, U, E>(self, f: F) -> Result<GuardStack<'a, (G, H), U>, E>
    where
        F: FnOnce(&'a T) -> Result<H, E>,
        H: Deref<Target = U>,
        U: 'a + ?Sized,
    {
        let new_guard = f(unsafe { std::mem::transmute(&*self.value) })?;

        Ok(GuardStack {
            value: SendPtr(&*new_guard),
            guards: (self.guards, new_guard),
            marker: PhantomData,
        })
    }

    pub async fn map_async<A, F, U, V>(self, f: F) -> GuardStack<'a, G, U>
    where
        A: Future<Output = V>,
        F: FnOnce(&'a T) -> A,
        U: 'a + ?Sized,
        V: Deref<Target = U>,
    {
        GuardStack {
            value: SendPtr(&*f(unsafe { std::mem::transmute(&*self.value) }).await),
            guards: self.guards,
            marker: PhantomData,
        }
    }

    pub async fn try_map_async<A, F, U, V, E>(self, f: F) -> Result<GuardStack<'a, G, U>, E>
    where
        A: Future<Output = Result<V, E>>,
        F: FnOnce(&'a T) -> A,
        U: 'a + ?Sized,
        V: Deref<Target = U>,
    {
        Ok(GuardStack {
            value: SendPtr(&*f(unsafe { std::mem::transmute(&*self.value) }).await?),
            guards: self.guards,
            marker: PhantomData,
        })
    }

    pub async fn push_guard_async<A, F, H, U>(self, f: F) -> GuardStack<'a, (G, H), U>
    where
        A: Future<Output = H>,
        F: FnOnce(&'a T) -> A,
        H: Deref<Target = U>,
        U: 'a + ?Sized,
    {
        let new_guard = f(unsafe { std::mem::transmute(&*self.value) }).await;

        GuardStack {
            value: SendPtr(&*new_guard),
            guards: (self.guards, new_guard),
            marker: PhantomData,
        }
    }

    pub async fn try_push_guard_async<A, F, H, U, E>(
        self,
        f: F,
    ) -> Result<GuardStack<'a, (G, H), U>, E>
    where
        A: Future<Output = Result<H, E>>,
        F: FnOnce(&'a T) -> A,
        H: Deref<Target = U>,
        U: 'a + ?Sized,
    {
        let new_guard = f(unsafe { std::mem::transmute(&*self.value) }).await?;

        Ok(GuardStack {
            value: SendPtr(&*new_guard),
            guards: (self.guards, new_guard),
            marker: PhantomData,
        })
    }
}

impl<'a, G, T> Deref for GuardStack<'a, G, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.value
    }
}

/// Lol, yolo.  This should be fine as long as you don't use it on anything bad.
struct SendPtr<T>(*const T)
where
    T: ?Sized;

unsafe impl<T> Send for SendPtr<T> where T: ?Sized {}
unsafe impl<T> Sync for SendPtr<T> where T: ?Sized {}

impl<T> Deref for SendPtr<T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
